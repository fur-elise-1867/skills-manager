import { useEffect, useState } from "react";
import { getCatalogTaxonomy } from "../api";
import type { CatalogQuery, Release, Taxonomy } from "../contracts";
import { storeErrorMessage } from "../errors";
import { useSdsSession } from "../session/context";
import SkillDetailPanel, { Compatibility } from "./SkillDetailPanel";
import useCatalog from "./useCatalog";

export default function CatalogPage({onInstall}:{onInstall?:(release:Release)=>void}) {
  const account = useSdsSession();
  const [query,setQuery] = useState<CatalogQuery>({q:"",mode:"all_time",sourceId:null,page:1,pageSize:20});
  const [draft,setDraft] = useState("");
  const [revision,setRevision] = useState(0);
  const [selected,setSelected] = useState<string>();
  const [taxonomy,setTaxonomy] = useState<Taxonomy>();
  const [taxonomyError,setTaxonomyError] = useState("");
  const [allSources,setAllSources] = useState(false);
  const result = useCatalog(query,revision);
  useEffect(() => {
    let active = true;
    getCatalogTaxonomy().then((data)=>{if(active) {setTaxonomy(data);setTaxonomyError("");}}).catch((error)=>{if(active) setTaxonomyError(storeErrorMessage(error));});
    return ()=>{active=false;};
  },[account.generation,revision]);
  useEffect(() => {
    const pending = setTimeout(()=>setQuery((current)=>current.q === draft.trim() ? current : {...current,q:draft.trim(),page:1}),300);
    return ()=>clearTimeout(pending);
  },[draft]);
  const retry = () => setRevision(revision+1);
  if (selected) return <SkillDetailPanel skillId={selected} onBack={()=>setSelected(undefined)} onInstall={onInstall} />;
  return <section className="sds-catalog" aria-label="Skills catalog">
    <div className="sds-store-tabs"><span aria-current="page">Store</span><button disabled title="Available in a later release">Local Install</button></div>
    <div className="sds-catalog-toolbar">
      <div className="sds-catalog-modes" aria-label="Catalog mode">{(taxonomy?.catalogModes ?? [{id:"all_time" as const,label:"All Time",available:true}]).map((mode)=><button key={mode.id} aria-pressed={query.mode === mode.id} disabled={!mode.available} title={mode.available ? undefined : "This mode is not available"} onClick={()=>setQuery({...query,mode:mode.id,page:1})}>{mode.label}</button>)}</div>
      <form role="search" onSubmit={(event)=>{event.preventDefault();setQuery((current)=>({...current,q:draft.trim(),page:1}));}}><label className="sds-visually-hidden" htmlFor="sds-catalog-search">Search skills</label><input id="sds-catalog-search" placeholder="Search skills by name or description…" value={draft} onChange={(event)=>setDraft(event.target.value)} /><button type="submit">Search</button></form>
    </div>
    <div className="sds-sources" role="group" aria-label="Source"><span>Source</span><button aria-pressed={query.sourceId === null} onClick={()=>setQuery({...query,sourceId:null,page:1})}>All sources</button>{(allSources ? taxonomy?.sources : taxonomy?.sources.slice(0,8))?.map((source)=><button key={source.id} aria-pressed={query.sourceId === source.id} onClick={()=>setQuery({...query,sourceId:source.id,page:1})}>{source.label}</button>)}{(taxonomy?.sources.length ?? 0) > 8 && <button aria-expanded={allSources} onClick={()=>setAllSources(!allSources)}>{allSources ? "Fewer sources" : "More sources…"}</button>}</div>
    {taxonomyError && <div className="sds-notice" role="alert"><p>{taxonomyError}</p><button onClick={retry}>Retry filters</button></div>}
    {result.error ? <div className="sds-notice" role="alert"><p>{result.error}</p><button onClick={retry}>Try again</button></div>
      : !result.data ? <p role="status">Loading skills…</p>
      : result.data.items.length === 0 ? <div className="sds-empty"><h2>No skills found</h2><p>Try another keyword or Source.</p></div>
      : <>
        <div className="sds-skill-grid">{result.data.items.map((skill)=><article className="sds-skill-card" key={skill.id}>
          <div className="sds-card-heading"><h2>{skill.name}</h2><span>{skill.latestRelease?.version ?? "No release"}</span></div><p className="sds-card-description">{skill.description}</p>
          <div className="sds-card-tags">{skill.roles.map((role)=><span key={role}>{role}</span>)}{skill.tags.slice(0,3).map((tag)=><span key={tag}>#{tag}</span>)}</div>
          <p className="sds-card-author">{skill.creator.displayName}</p><Compatibility agents={skill.compatibleAgents} />
          <button className="sds-text-button" aria-label={`View ${skill.name}`} onClick={()=>setSelected(skill.id)}>View skill →</button>
        </article>)}</div>
        <div className="sds-pagination"><span>{result.data.totalItems.toLocaleString()} {result.data.totalItems === 1 ? "skill" : "skills"} · Page {query.page}</span><div><button disabled={query.page === 1} aria-label="Previous page" onClick={()=>setQuery({...query,page:query.page-1})}>←</button><button disabled={query.page * query.pageSize >= result.data.totalItems} aria-label="Next page" onClick={()=>setQuery({...query,page:query.page+1})}>→</button></div></div>
      </>}
  </section>;
}
