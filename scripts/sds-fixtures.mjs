// Explicit development-only loopback server. The installed app never starts it.
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';

const readFixture = (name) => JSON.parse(readFileSync(new URL(`../contracts/sds-desktop/v1/fixtures/${name}.json`,import.meta.url),'utf8'));
const success = (data) => ({success:true,data});
const error = (errorCode) => ({success:false,errorCode,message:'Synthetic development response'});

export async function startFixtureStore() {
  const login = readFixture('auth-login');
  const refresh = readFixture('auth-refresh');
  const taxonomy = readFixture('taxonomy');
  const sample = readFixture('catalog').data.items[0];
  const release = readFixture('release').data;
  const detail = readFixture('detail').data;
  const skills = [
    sample,
    ...['Meeting Notes','Code Review Checklist','Confluence Summary','Excel Status Report','Release Notes'].map((name,index)=>({
      ...structuredClone(sample),id:`sample-${index+1}`,name,description:`Sample ${name.toLowerCase()} workflow for internal teams.`,
      roles:index%2 ? ['DEV'] : ['PM'],tags:['sample'],sourceIds:index%2 ? ['security'] : ['quality'],latestRelease:null,
    })),
  ];
  const server = createServer(async(req,res)=>{
    const send = (status,body)=>{res.writeHead(status,{'content-type':'application/json','cache-control':'no-store'});res.end(JSON.stringify(body));};
    try {
      const url = new URL(req.url,'http://127.0.0.1');
      const path = url.pathname;
      if(!path.startsWith('/skm/api/v1/')) return send(404,error('NOT_FOUND'));
      if(path.startsWith('/skm/api/v1/auth/') && req.method === 'POST') {
        let body = ''; for await(const chunk of req) { body += chunk; if(Buffer.byteLength(body)>65536) return send(413,error('INVALID_REQUEST')); }
        const data = JSON.parse(body || '{}');
        if(path.endsWith('/login')) return send(data.email === 'employee@example.invalid' && data.password === 'synthetic-password' ? 200 : 400, data.email === 'employee@example.invalid' && data.password === 'synthetic-password' ? login : error('INVALID_PASSWORD'));
        if(path.endsWith('/refresh')) return send(['synthetic-refresh-one','synthetic-refresh-two'].includes(data.refreshToken) ? 200 : 401, ['synthetic-refresh-one','synthetic-refresh-two'].includes(data.refreshToken) ? refresh : error('SESSION_EXPIRED'));
        if(path.endsWith('/logout')) return send(200,success(null));
      }
      if(req.method !== 'GET') return send(405,error('INVALID_REQUEST'));
      if(!['Bearer synthetic-access-one','Bearer synthetic-access-two'].includes(req.headers.authorization)) return send(401,error('SESSION_EXPIRED'));
      if(path === '/skm/api/v1/desktop/taxonomy') return send(200,taxonomy);
      if(path === '/skm/api/v1/desktop/skills') {
        const mode = url.searchParams.get('mode') ?? 'all_time';
        if(!['all_time','trending','hot'].includes(mode)) return send(400,error('INVALID_REQUEST'));
        if(mode !== 'all_time') return send(422,error('CATALOG_MODE_UNAVAILABLE'));
        const q = (url.searchParams.get('q') ?? '').trim().toLowerCase();
        if(q === '__unavailable') return send(503,error('STORE_UNAVAILABLE'));
        if(q === '__slow') await new Promise(resolve=>setTimeout(resolve,1500));
        const source = url.searchParams.get('sourceId');
        const page = Number(url.searchParams.get('page') ?? '1');
        const pageSize = Number(url.searchParams.get('pageSize') ?? '20');
        if(!Number.isInteger(page) || page<1 || !Number.isInteger(pageSize) || pageSize<1 || pageSize>100 || source && !taxonomy.data.sources.some(item=>item.id === source)) return send(400,error('INVALID_REQUEST'));
        const filtered = skills.filter(skill=>(!q || q === '__slow' || `${skill.name} ${skill.description}`.toLowerCase().includes(q)) && (!source || skill.sourceIds.includes(source)))
          .sort((left,right)=>left.name.localeCompare(right.name,'en',{sensitivity:'base'}) || left.id.localeCompare(right.id));
        return send(200,success({items:filtered.slice((page-1)*pageSize,page*pageSize),page,pageSize,totalItems:filtered.length}));
      }
      const match = /^\/skm\/api\/v1\/desktop\/skills\/([^/]+)(?:\/releases\/([^/]+))?$/.exec(path);
      if(match) {
        const skill = skills.find(skill=>skill.id === match[1]);
        if(!skill) return send(404,error('NOT_FOUND'));
        if(match[2]) return send(match[2] === release.id && skill.id === sample.id ? 200 : 404, match[2] === release.id && skill.id === sample.id ? success(release) : error('NOT_FOUND'));
        return send(200,success(skill.id === sample.id ? detail : {...skill,storePageUrl:null,latestRelease:null}));
      }
      return send(404,error('NOT_FOUND'));
    } catch { if(!res.headersSent) send(400,error('INVALID_REQUEST')); else res.destroy(); }
  });
  server.requestTimeout = 5000;
  server.headersTimeout = 5000;
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
  return {baseUrl:`http://127.0.0.1:${server.address().port}/skm`,close:()=>new Promise((resolve,reject)=>{server.close(error=>error ? reject(error) : resolve());server.closeAllConnections();})};
}
