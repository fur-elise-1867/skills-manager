import {act,fireEvent,render,screen,waitFor} from "@testing-library/react";
import {beforeEach,expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import OperationFlow from "./OperationFlow";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";
import type {PreparedOperation,Release} from "../contracts";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/event",()=>({listen:vi.fn(async()=>vi.fn())}));
const plan:PreparedOperation={id:"operation-one",kind:"install",release:release.data as Release,target:{agentKey:"aipro",workspaceId:"global",path:"C:\\Synthetic\\skills\\jira-weekly-report"},expectedFingerprint:null,mcp:{state:"missing",missing:["jira-mcp"],reason:null,checkedAt:""},requiresMcpAck:true,canExecute:true,conflict:null};
let completed=false;
beforeEach(()=>{completed=false;vi.mocked(invoke).mockImplementation(async command=>{
  if(command==="sds_prepare_install")return plan as never;
  if(command==="sds_execute")return {operationId:plan.id} as never;
  if(command==="sds_operation_status")return {operationId:plan.id,state:completed?"completed":"downloading",stage:completed?"completed":"downloading",error:null} as never;
  if(command==="sds_cancel")return {operationId:plan.id,state:"cancelled",stage:"cancelled",error:null} as never;
  throw new Error(`Unexpected ${command}`);
});});
test("reviews MCP then uses authoritative status to finish even when progress events are missed",async()=>{
  const close=vi.fn(),refresh=vi.fn();render(<OperationFlow request={{kind:"install",skillId:"skill-123",releaseId:"rel-003"}} onClose={close} onCompleted={refresh}/>);
  await screen.findByRole("button",{name:"Install anyway"});
  fireEvent.click(screen.getByRole("button",{name:"Install anyway"}));
  expect(await screen.findByRole("heading",{name:"Installing skill"})).toBeInTheDocument();
  expect(screen.queryByText("Installed successfully")).not.toBeInTheDocument();
  completed=true;
  expect(await screen.findByText("Installed successfully",{}, {timeout:3000})).toBeInTheDocument();
  expect(refresh).toHaveBeenCalledOnce();
  expect(invoke).toHaveBeenCalledWith("sds_execute",{operationId:plan.id,ackMcp:true});
  expect(vi.mocked(invoke).mock.calls.filter(([command])=>command==="sds_execute")).toHaveLength(1);
  fireEvent.click(screen.getByRole("button",{name:"Done"}));expect(close).toHaveBeenCalledOnce();
});
test("late preparation from an earlier request cannot replace the current review",async()=>{
  let resolveFirst!:(value:PreparedOperation)=>void;
  vi.mocked(invoke).mockImplementation(async(command,args)=>{
    if(command==="sds_prepare_install" && (args as {request:{skillId:string}}).request.skillId==="old")return new Promise(resolve=>{resolveFirst=resolve;}) as never;
    return {...plan,id:"new-plan"} as never;
  });
  const props={onClose:vi.fn(),onCompleted:vi.fn()};
  const view=render(<OperationFlow {...props} request={{kind:"install",skillId:"old",releaseId:"rel-003"}}/>);
  view.rerender(<OperationFlow {...props} request={{kind:"install",skillId:"new",releaseId:"rel-003"}}/>);
  await screen.findByRole("button",{name:"Install anyway"});
  await act(async()=>resolveFirst({...plan,release:{...plan.release,installName:"old-result"}}));
  expect(screen.queryByText(/old-result/)).not.toBeInTheDocument();
});
test("remove enters the same execution/status flow without asking for MCP or deleting Library",async()=>{
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_prepare_remove")return {...plan,kind:"remove",requiresMcpAck:false,expectedFingerprint:"a".repeat(64)} as never;
    if(command==="sds_execute")return {operationId:plan.id} as never;
    return {operationId:plan.id,state:"completed",stage:"removed",error:null} as never;
  });
  render(<OperationFlow request={{kind:"remove",deploymentId:"deployment-one"}} onClose={vi.fn()} onCompleted={vi.fn()}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Remove from target"}));
  expect(await screen.findByText("Removed from target")).toBeInTheDocument();
  expect(invoke).toHaveBeenCalledWith("sds_execute",{operationId:plan.id,ackMcp:false});
});
test("a failed download remains an explicit failure instead of installation success",async()=>{
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_prepare_install")return plan as never;
    if(command==="sds_execute")return {operationId:plan.id} as never;
    return {operationId:plan.id,state:"failed",stage:"failed",error:{code:"PACKAGE_INTEGRITY",message:"synthetic-secret"}} as never;
  });
  render(<OperationFlow request={{kind:"install",skillId:"skill-123",releaseId:"rel-003"}} onClose={vi.fn()} onCompleted={vi.fn()}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Install anyway"}));
  await waitFor(()=>expect(screen.getByRole("alert")).toHaveTextContent("checksum"));
  expect(screen.queryByText("Installed successfully")).not.toBeInTheDocument();
  expect(screen.queryByText("synthetic-secret")).not.toBeInTheDocument();
});

test("removal waits for cleanup and displays the authoritative retained-files warning",async()=>{
  let cleaned=false;
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_prepare_remove")return {...plan,kind:"remove",requiresMcpAck:false,expectedFingerprint:"a".repeat(64)} as never;
    if(command==="sds_execute")return {operationId:plan.id} as never;
    return {operationId:plan.id,state:"completed",stage:cleaned?"removed-files-kept":"removal-cleanup",error:cleaned?{code:"CLEANUP_REVIEW_REQUIRED",message:"synthetic-secret"}:null} as never;
  });
  const refresh=vi.fn();
  render(<OperationFlow request={{kind:"remove",deploymentId:"deployment-one"}} onClose={vi.fn()} onCompleted={refresh}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Remove from target"}));
  await screen.findByRole("heading",{name:"Removing skill"});
  expect(screen.queryByRole("button",{name:"Done"})).not.toBeInTheDocument();
  expect(refresh).not.toHaveBeenCalled();
  cleaned=true;
  expect(await screen.findByRole("alert",{}, {timeout:3000})).toHaveTextContent(/retained|kept/i);
  expect(await screen.findByRole("button",{name:"Done"})).toBeInTheDocument();
  expect(refresh).toHaveBeenCalledOnce();
});
