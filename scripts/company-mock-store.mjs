import { createServer } from 'node:http';
import { createHash, randomUUID } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { readFileSync } from 'node:fs';
import { join, basename } from 'node:path';

export function createMockStore({ scenario = 'normal' } = {}) {
  const sessions = new Map();
  let metadataChanged = false;
  const fixtureRoot = fileURLToPath(new URL('../fixtures/company-store/', import.meta.url));
  function catalog() {
    return JSON.parse(readFileSync(join(fixtureRoot, 'catalog.json'), 'utf8')).map(entry => {
      const bytes = readFileSync(join(fixtureRoot, 'packages', basename(entry.archive)));
      const sha256 = createHash('sha256').update(bytes).digest('hex');
      const description = metadataChanged ? entry.description.replace(/MCP bắt buộc: [^\n]*/, 'MCP bắt buộc: github') : entry.description;
      const { archive, ...skill } = entry;
      return { ...skill, description, package: { format: archive.endsWith('.7z') ? '7z' : 'zip', size_bytes: bytes.length, sha256, etag: `"${createHash('sha256').update(sha256 + description).digest('hex')}"` }, bytes };
    });
  }
  return createServer(async (req, res) => {
    const url = new URL(req.url, 'http://127.0.0.1');
    const error = (status, code, message) => {
      res.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
      res.end(JSON.stringify({ error: { code, message, request_id: randomUUID() } }));
    };
    const ok = data => {
      res.writeHead(200, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
      res.end(JSON.stringify({ data }));
    };
    if (scenario === 'unavailable') return error(503, 'STORE_UNAVAILABLE', 'Skill Store chưa khả dụng.');
    if (req.method === 'POST' && url.pathname === '/api/v1/auth/login') {
      try {
        let body = '';
        for await (const chunk of req) { body += chunk; if (body.length > 16384) return error(400, 'VALIDATION_ERROR', 'Request quá lớn.'); }
        const input = JSON.parse(body);
        if (!['employee01', 'employee02'].includes(input.username) || input.password !== 'demo-password') return error(401, 'INVALID_CREDENTIALS', 'Tài khoản hoặc mật khẩu không đúng.');
        const user = { id: input.username === 'employee01' ? 'usr_001' : 'usr_002', username: input.username, display_name: input.username === 'employee01' ? 'Nhân viên 01' : 'Nhân viên 02' };
        const token = randomUUID();
        const expires_at = new Date(Date.now() + 12 * 3600 * 1000).toISOString();
        sessions.set(token, { user, expires_at });
        return ok({ access_token: token, token_type: 'Bearer', expires_at, user });
      } catch { return error(400, 'VALIDATION_ERROR', 'JSON không hợp lệ.'); }
    }
    const token = req.headers.authorization?.match(/^Bearer (\S+)$/)?.[1];
    if (req.method === 'POST' && url.pathname === '/api/v1/auth/logout') {
      if (!token) return error(401, 'UNAUTHENTICATED', 'Thiếu Bearer token.');
      sessions.delete(token); res.writeHead(204); return res.end();
    }
    if (!token || !sessions.has(token)) return error(401, 'UNAUTHENTICATED', 'Cần đăng nhập Skill Store.');
    if (scenario === 'session-expired' || Date.parse(sessions.get(token).expires_at) <= Date.now()) return error(401, 'SESSION_EXPIRED', 'Phiên đăng nhập đã hết hạn.');
    if (req.method === 'GET' && url.pathname === '/api/v1/skills') {
      const query = (url.searchParams.get('q') || '').trim();
      const page = Number(url.searchParams.get('page') || 1);
      const pageSize = Number(url.searchParams.get('page_size') || 20);
      if (query.length > 200 || !Number.isInteger(page) || page < 1 || !Number.isInteger(pageSize) || pageSize < 1 || pageSize > 100) return error(400, 'VALIDATION_ERROR', 'Phân trang hoặc từ khóa không hợp lệ.');
      const matches = catalog().filter(item => (item.name + item.description).toLocaleLowerCase().includes(query.toLocaleLowerCase())).sort((a, b) => b.updated_at.localeCompare(a.updated_at) || a.id.localeCompare(b.id));
      const items = matches.slice((page - 1) * pageSize, page * pageSize).map(({ bytes: _bytes, description: _description, ...item }) => item);
      return ok({ items, pagination: { page, page_size: pageSize, total: matches.length, total_pages: Math.ceil(matches.length / pageSize) } });
    }
    const match = url.pathname.match(/^\/api\/v1\/skills\/([^/]+)(\/package)?$/);
    if (req.method === 'GET' && match) {
      if (match[2] && scenario === 'package-changed' && !metadataChanged) metadataChanged = true;
      const item = catalog().find(entry => entry.id === decodeURIComponent(match[1]));
      if (!item) return error(404, 'SKILL_NOT_FOUND', 'Không tìm thấy skill.');
      const { bytes, ...detail } = item;
      if (!match[2]) return ok(detail);
      if (scenario === 'forbidden-download') return error(403, 'FORBIDDEN', 'Tài khoản không được phép tải gói.');
      if (!req.headers['if-match'] || req.headers['if-match'] === '*' || req.headers['if-match'].startsWith('W/')) return error(400, 'VALIDATION_ERROR', 'Cần strong ETag từ detail.');
      if (req.headers['if-match'] !== item.package.etag) return error(412, 'PACKAGE_CHANGED', 'Thông tin hoặc gói skill đã thay đổi. Vui lòng lấy lại chi tiết.');
      const payload = Buffer.from(bytes);
      if (scenario === 'corrupt-download') payload[payload.length - 1] ^= 1;
      res.writeHead(200, { 'Content-Type': item.package.format === 'zip' ? 'application/zip' : 'application/x-7z-compressed', 'Content-Length': payload.length, 'Content-Disposition': `attachment; filename="${item.slug}.${item.package.format}"`, ETag: item.package.etag, 'Cache-Control': 'private, no-store, no-transform' });
      return res.end(payload);
    }
    return error(404, 'SKILL_NOT_FOUND', 'Không tìm thấy skill.');
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const value = name => { const i = process.argv.indexOf(name); return i >= 0 ? process.argv[i + 1] : undefined; };
  const port = Number(value('--port') || 4318);
  const server = createMockStore({ scenario: value('--scenario') || 'normal' });
  server.listen(port, '127.0.0.1', () => console.log(`Mock Skill Store: http://127.0.0.1:${server.address().port} (employee01 / demo-password)`));
  process.on('SIGTERM', () => server.close());
}
