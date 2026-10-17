import {useCallback,useEffect,useLayoutEffect,useRef,useState} from "react";
import {checkUpdates} from "../api";
import type {UpdateCheck} from "../contracts";
import {storeErrorMessage} from "../errors";
import {useSdsSession} from "../session/context";

export default function useUpdateCheck() {
  const {connection,session,generation,refreshStatus}=useSdsSession();
  const status=session?.status;
  const key=JSON.stringify([connection?.id,session?.user?.id,generation,status]);
  const current=useRef(key);
  useLayoutEffect(()=>{current.current=key;return()=>{current.current="";};},[key]);
  const seen=useRef(new Set<string>()),requestId=useRef(0);
  const [state,setState]=useState<{key:string;data?:UpdateCheck;error?:string;busy?:boolean}>({key:""});
  const refresh=useCallback(async()=>{
    if(status!=="authenticated" || !connection?.id)return;
    await Promise.resolve();
    if(current.current!==key)return;
    const id=++requestId.current;setState({key,busy:true});
    try{const data=await checkUpdates();if(current.current===key && requestId.current===id)setState({key,data});}
    catch(failure){if(current.current===key && requestId.current===id){setState({key,error:storeErrorMessage(failure)});if(typeof failure==="object" && failure!==null && "code" in failure && ["SESSION_EXPIRED","STORE_UNAVAILABLE"].includes(String(failure.code)))void refreshStatus();}}
  },[key,status,connection?.id,refreshStatus]);
  useEffect(()=>{if(status==="authenticated" && connection?.id && !seen.current.has(key)){seen.current.add(key);void Promise.resolve().then(refresh);}},[key,status,connection?.id,refresh]);
  return {...(state.key===key?state:{}),refresh};
}

