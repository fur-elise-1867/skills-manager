import { useEffect, useState } from "react";
import { searchCatalog } from "../api";
import type { CatalogQuery, Page, SkillSummary } from "../contracts";
import { storeErrorMessage } from "../errors";
import { useSdsSession } from "../session/context";

export default function useCatalog(query: CatalogQuery, revision: number) {
  const { generation, connection, session, refreshStatus } = useSdsSession();
  const key = JSON.stringify([connection?.id, session?.user?.id, generation, query, revision]);
  const [result, setResult] = useState<{key:string; data?:Page<SkillSummary>; error?:string}>({key:""});
  useEffect(() => {
    let active = true;
    searchCatalog(query).then((data) => { if (active) setResult({key,data}); }).catch((failure) => {
      if (!active) return;
      setResult({key,error:storeErrorMessage(failure)});
      if (failure?.code === "SESSION_EXPIRED" || failure?.code === "STORE_UNAVAILABLE") void refreshStatus();
    });
    return () => { active = false; };
  }, [key, query, refreshStatus]);
  return result.key === key ? result : {key};
}
