import {fireEvent,render,screen,waitFor} from "@testing-library/react";
import {beforeEach,expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import LibraryPage from "../library/LibraryPage";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/event",()=>({listen:vi.fn(async()=>vi.fn())}));
const base={id:"atlas",target:{agentKey:"aipro",workspaceId:"atlas",path:"D:\\Atlas\\.aipro\\skills\\jira-weekly-report"},status:"modified",mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},installedAt:"",reason:null};
const item={id:"release-key",profileId:"profile-one",release:release.data,cachedAt:"2026-10-17T00:00:00Z",cacheStatus:"verified",deployments:[base,{...base,id:"global",target:{...base.target,workspaceId:"global",path:"C:\\Home\\.aipro\\skills\\jira-weekly-report"},status:"installed"}]};
const updates={checkedAt:"2026-10-17T00:00:00Z",items:[{skillId:release.data.skillId,currentReleaseId:release.data.id,status:"update_available" as const,latestRelease:{id:"release-new",sequence:4,version:"1.3.0"}}]};
beforeEach(()=>{vi.mocked(invoke).mockImplementation(async command=>{
  if(command==="sds_library")return [item] as never;
  if(command==="sds_check_updates")return updates as never;
  if(command==="sds_prepare_updates")return {selectionId:"selected-one",items:[{deploymentId:"atlas",operationId:"update-atlas",operation:{id:"update-atlas",kind:"update",release:{...release.data,id:"release-new",version:"1.3.0",sequence:4,changelog:"Adds blocked-task summaries"},target:base.target,expectedFingerprint:"edited",mcp:{state:"missing",missing:["new-mcp"],reason:null,checkedAt:""},requiresMcpAck:true,canExecute:true,conflict:null}}]} as never;
  throw new Error(`Unexpected ${command}`);
});});

test("update metadata does not install and only the explicitly selected target enters review",async()=>{
  const operation=vi.fn();render(<LibraryPage session={{status:"authenticated",user:{id:"u",displayName:"Employee"}}} profileId="profile-one" updates={updates} onOperation={operation}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Review update"}));
  expect(screen.getByRole("button",{name:"Review selected targets"})).toBeDisabled();
  fireEvent.click(screen.getByLabelText("Update atlas"));
  expect(screen.getByText(/If backup fails, this copy will not be updated/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Review selected targets"}));
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith("sds_prepare_updates",{requests:[{deploymentId:"atlas",releaseId:"release-new",localEdits:"backupAndReplace"}]}));
  expect(await screen.findByText("Adds blocked-task summaries")).toBeInTheDocument();
  expect(screen.getByText("new-mcp")).toBeInTheDocument();
  expect(screen.getByRole("button",{name:"Back up and update selected targets"})).toBeDisabled();
  expect(operation).not.toHaveBeenCalled();
  expect(invoke).not.toHaveBeenCalledWith("sds_execute",expect.anything());
});

test("a partial group failure retries only the failed target after fresh review",async()=>{
  const requests:unknown[]=[];
  let preparation=0;
  vi.mocked(invoke).mockImplementation(async(command,args)=>{
    if(command==="sds_library")return [item] as never;
    if(command==="sds_prepare_updates"){
      requests.push(args);preparation++;
      const ids=preparation===1?["atlas","global"]:["global"];
      return {selectionId:`selection-${preparation}`,items:ids.map(id=>({deploymentId:id,operationId:`${id}-${preparation}`,operation:{id:`${id}-${preparation}`,kind:"update",release:{...release.data,id:"release-new",sequence:4,version:"1.3.0",changelog:"New release"},target:item.deployments.find(deployment=>deployment.id===id)!.target,expectedFingerprint:"observed",mcp:{state:"missing",missing:["new-mcp"],reason:null,checkedAt:""},requiresMcpAck:true,canExecute:true,conflict:null}}))} as never;
    }
    if(command==="sds_execute_updates")return {selectionId:`selection-${preparation}`} as never;
    if(command==="sds_update_selection_status")return {selectionId:`selection-${preparation}`,items:preparation===1?[{deploymentId:"atlas",operationId:"atlas-1",result:{operationId:"atlas-1",state:"completed",stage:"completed",error:null}},{deploymentId:"global",operationId:"global-1",result:{operationId:"global-1",state:"failed",stage:"failed",error:{code:"REVIEW_REQUIRED",message:"unsafe provider error"}}}]:[{deploymentId:"global",operationId:"global-2",result:{operationId:"global-2",state:"completed",stage:"completed",error:null}}]} as never;
    throw new Error(`Unexpected ${command}`);
  });
  render(<LibraryPage profileId="profile-one" updates={updates} onOperation={vi.fn()}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Review update"}));
  fireEvent.click(screen.getByLabelText("Update atlas"));fireEvent.click(screen.getByLabelText("Update global"));
  fireEvent.click(screen.getByRole("button",{name:"Review selected targets"}));
  fireEvent.click(await screen.findByLabelText("Acknowledge MCP warnings for atlas"));
  expect(screen.getByRole("button",{name:"Back up and update selected targets"})).toBeDisabled();
  fireEvent.click(screen.getByLabelText("Acknowledge MCP warnings for global"));
  fireEvent.click(screen.getByRole("button",{name:"Back up and update selected targets"}));
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith("sds_execute_updates",{selectionId:"selection-1",acknowledgedOperationIds:["atlas-1","global-1"]}));
  fireEvent.click(await screen.findByRole("button",{name:"Review failed targets again"}));
  await waitFor(()=>expect(requests[1]).toEqual({requests:[{deploymentId:"global",releaseId:"release-new",localEdits:"backupAndReplace"}]}));
  expect(screen.getByLabelText("Acknowledge MCP warnings for global")).not.toBeChecked();
  fireEvent.click(screen.getByLabelText("Acknowledge MCP warnings for global"));
  fireEvent.click(screen.getByRole("button",{name:"Back up and update selected targets"}));
  expect(await screen.findByText("All selected updates completed.")).toBeInTheDocument();
  expect(screen.queryByText("unsafe provider error")).not.toBeInTheDocument();
  expect(vi.mocked(invoke).mock.calls.filter(([name])=>name==="sds_execute_updates")).toHaveLength(2);
});

test("Keep current cancels update selection without downloading",async()=>{
  const operation=vi.fn();render(<LibraryPage updates={updates} profileId="profile-one" onOperation={operation}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Review update"}));
  fireEvent.click(screen.getByRole("button",{name:"Keep current"}));
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();expect(operation).not.toHaveBeenCalled();
});

test("a failed retry preparation cannot reselect or replay a successful target",async()=>{
  let preparation=0;const payloads:unknown[]=[];
  vi.mocked(invoke).mockImplementation(async(command,args)=>{
    if(command==="sds_library")return [item] as never;
    if(command==="sds_prepare_updates"){
      payloads.push(args);preparation++;
      if(preparation===2)throw {code:"STORE_UNAVAILABLE"};
      return {selectionId:"selection-one",items:(args as {requests:{deploymentId:string}[]}).requests.map(request=>({deploymentId:request.deploymentId,operationId:request.deploymentId,operation:{id:request.deploymentId,kind:"update",release:{...release.data,id:"release-new",sequence:4},target:base.target,expectedFingerprint:"current",mcp:base.mcp,requiresMcpAck:false,canExecute:true,conflict:null}}))} as never;
    }
    if(command==="sds_execute_updates")return {selectionId:"selection-one"} as never;
    if(command==="sds_update_selection_status")return {selectionId:"selection-one",items:[{deploymentId:"atlas",operationId:"atlas",result:{operationId:"atlas",state:"completed",stage:"completed",error:null}},{deploymentId:"global",operationId:"global",result:{operationId:"global",state:"failed",stage:"failed",error:{code:"REVIEW_REQUIRED"}}}]} as never;
    throw new Error(`Unexpected ${command}`);
  });
  render(<LibraryPage profileId="profile-one" updates={updates} onOperation={vi.fn()}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Review update"}));
  fireEvent.click(screen.getByLabelText("Update atlas"));fireEvent.click(screen.getByLabelText("Update global"));
  fireEvent.click(screen.getByRole("button",{name:"Review selected targets"}));
  fireEvent.click(await screen.findByRole("button",{name:"Back up and update selected targets"}));
  fireEvent.click(await screen.findByRole("button",{name:"Review failed targets again"}));
  await screen.findByRole("alert");
  expect(screen.getByLabelText("Update atlas")).not.toBeChecked();
  expect(screen.getByLabelText("Update atlas")).toBeDisabled();
  fireEvent.click(screen.getByRole("button",{name:"Review selected targets"}));
  await waitFor(()=>expect(payloads[2]).toEqual({requests:[{deploymentId:"global",releaseId:"release-new",localEdits:"backupAndReplace"}]}));
});

