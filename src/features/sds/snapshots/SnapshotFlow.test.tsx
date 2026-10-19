import {fireEvent,render,screen} from "@testing-library/react";
import {beforeEach,expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import SnapshotsPage from "./SnapshotsPage";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/event",()=>({listen:vi.fn(async()=>vi.fn())}));
const target={agentKey:"aipro",workspaceId:"atlas",path:"D:\\Atlas\\.aipro\\skills\\jira-weekly-report"};
beforeEach(()=>{vi.mocked(invoke).mockImplementation(async command=>{
  if(command==="runtime_profile")return {kind:"sds",environment:"internal",fixtureMode:false} as never;
  if(command==="sds_get_connection")return null as never;
  if(command==="sds_session_status")return {status:"signedOut",user:null} as never;
  if(command==="sds_get_target_settings")return {overrideRoot:null,target:null,error:null,locked:false} as never;
  if(command==="sds_library")return [] as never;
  if(command==="sds_snapshots")return [{id:"snapshot-one",deploymentId:"old-deployment",profileId:"old-profile",release:release.data,target,capturedAt:"2026-10-17T00:00:00Z",reason:"beforeUpdate",contentManifest:{entries:[{path:"SKILL.md",sizeBytes:123,sha256:"x"}]},contentFingerprint:"edited",baselineManifest:{entries:[]},baselineFingerprint:"original"}] as never;
  if(command==="sds_prepare_restore")return {id:"restore-one",kind:"restore",release:release.data,target,expectedFingerprint:"current",mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},requiresMcpAck:false,canExecute:true,conflict:null} as never;
  if(command==="sds_execute")return {operationId:"restore-one"} as never;
  if(command==="sds_operation_status")return {operationId:"restore-one",state:"completed",stage:"completed",error:null} as never;
  if(command==="sds_cancel")return {operationId:"restore-one",state:"cancelled",stage:"cancelled",error:null} as never;
  throw new Error(`Unexpected ${command}`);
});});

test("signed-out user restores a self-contained snapshot to its recorded target with current protection",async()=>{
  const onRestore=vi.fn();
  render(<SnapshotsPage onRestore={onRestore}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Restore…"}));
  expect(onRestore).toHaveBeenCalledWith({kind:"restore",snapshotId:"snapshot-one",deploymentId:"old-deployment",protectCurrent:true});
});

test("snapshot list reports corrupt bytes and blocks restore after explicit verification",async()=>{
  const previous=vi.mocked(invoke).getMockImplementation()!;
  vi.mocked(invoke).mockImplementation(async(command,args)=>{
    if(command==="sds_verify_snapshot")throw {code:"SNAPSHOT_CORRUPT"};
    return previous(command,args) as never;
  });
  render(<SnapshotsPage onRestore={vi.fn()}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Verify snapshot"}));
  expect(await screen.findByText("Corrupt or missing")).toBeInTheDocument();
  expect(screen.getByRole("button",{name:"Restore…"})).toBeDisabled();
  expect(invoke).toHaveBeenCalledWith("sds_verify_snapshot",{snapshotId:"snapshot-one"});
  expect(invoke).not.toHaveBeenCalledWith("sds_prepare_restore",expect.anything());
});
