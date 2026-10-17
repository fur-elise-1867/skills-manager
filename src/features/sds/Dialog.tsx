import {useEffect,useRef} from "react";
import type {ReactNode} from "react";

export default function Dialog({title,onClose,busy=false,children,wide=false}:{title:string;onClose:()=>void;busy?:boolean;children:ReactNode;wide?:boolean}) {
  const ref=useRef<HTMLDivElement>(null);
  useEffect(()=>{const previous=document.activeElement;ref.current?.focus();return()=>{if(previous instanceof HTMLElement && previous.isConnected)previous.focus();};},[]);
  return <div className="sds-modal-backdrop"><div ref={ref} tabIndex={-1} className={`sds-install-review ${wide?"sds-wide-review":""}`} role="dialog" aria-modal="true" aria-label={title} onKeyDown={event=>{
    if(event.key==="Escape" && !busy){event.preventDefault();onClose();}
    if(event.key!=="Tab")return;
    const controls=Array.from(ref.current?.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),a[href],[tabindex="0"]')??[]);
    const first=controls[0],last=controls.at(-1);
    if(!first){event.preventDefault();return;}
    if(event.shiftKey && (document.activeElement===first || document.activeElement===ref.current)){event.preventDefault();last?.focus();}
    else if(!event.shiftKey && (document.activeElement===last || document.activeElement===ref.current)){event.preventDefault();first.focus();}
  }}><h2>{title}</h2>{children}</div></div>;
}
