import {act,fireEvent,render,screen,waitFor} from "@testing-library/react";
import {beforeEach,expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import OperationFlow from "../install/OperationFlow";
import WorkspacePage from "./WorkspacePage";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";
import {open} from "@tauri-apps/plugin-dialog";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/event",()=>({listen:vi.fn(async()=>vi.fn())}));
vi.mock("@tauri-apps/plugin-dialog",()=>({open:vi.fn()}));
beforeEach(()=>{vi.mocked(invoke).mockImplementation(async(command,args)=>{
  if(command==="sds_list_workspaces")return [{id:"atlas",root:"D:\\Projects\\Atlas"}] as never;
  if(command==="sds_prepare_install")return {id:"install-atlas",kind:"install",release:release.data,target:{agentKey:"aipro",workspaceId:(args as {request:{target:{workspaceId:string}}}).request.target?.workspaceId??"global",path:"D:\\Projects\\Atlas\\.aipro\\skills\\jira-weekly-report"},mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},requiresMcpAck:false,canExecute:true,conflict:null,expectedFingerprint:null} as never;
  throw new Error(`Unexpected ${command}`);
});});

test("chooses a registered project before preparing a reviewed install and sends no destination path",async()=>{
  render(<OperationFlow chooseTarget request={{kind:"install",skillId:"skill-123",releaseId:"rel-003"}} onClose={vi.fn()} onCompleted={vi.fn()}/>);
  fireEvent.click(await screen.findByLabelText("Project"));
  fireEvent.change(await screen.findByLabelText("Registered project"),{target:{value:"atlas"}});
  expect(invoke).not.toHaveBeenCalledWith("sds_prepare_install",expect.anything());
  fireEvent.click(screen.getByRole("button",{name:"Review requirements"}));
  await screen.findByRole("heading",{name:"Install skill"});
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith("sds_prepare_install",{request:{skillId:"skill-123",releaseId:"rel-003",target:{agentKey:"aipro",workspaceId:"atlas"}}}));
  expect(screen.getByText("AIPro · Project")).toBeInTheDocument();
});

test("a changed install request cannot inherit the previous project review",async()=>{
  const props={chooseTarget:true,onClose:vi.fn(),onCompleted:vi.fn()};
  const view=render(<OperationFlow {...props} request={{kind:"install",skillId:"first",releaseId:"r1"}}/>);
  fireEvent.click(await screen.findByLabelText("Project"));
  fireEvent.change(await screen.findByLabelText("Registered project"),{target:{value:"atlas"}});
  fireEvent.click(screen.getByRole("button",{name:"Review requirements"}));
  await screen.findByRole("heading",{name:"Install skill"});
  view.rerender(<OperationFlow {...props} request={{kind:"install",skillId:"second",releaseId:"r2"}}/>);
  expect(await screen.findByRole("dialog",{name:"Choose installation location"})).toBeInTheDocument();
});

test("workspace view shows only copies in the selected registered project",async()=>{
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_list_workspaces")return [{id:"atlas",root:"D:\\Projects\\Atlas"}] as never;
    if(command==="sds_library")return [{id:"release-key",profileId:"profile-one",release:release.data,cachedAt:"",cacheStatus:"verified",deployments:[{id:"atlas-deployment",target:{agentKey:"aipro",workspaceId:"atlas",path:"D:\\Projects\\Atlas\\.aipro\\skills\\jira-weekly-report"},status:"modified",mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},installedAt:"",reason:null},{id:"global-deployment",target:{agentKey:"aipro",workspaceId:"global",path:"C:\\Home\\.aipro\\skills\\jira-weekly-report"},status:"installed",mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},installedAt:"",reason:null}]}] as never;
    throw new Error(`Unexpected ${command}`);
  });
  render(<WorkspacePage onOperation={vi.fn()}/>);
  fireEvent.change(await screen.findByLabelText("Registered project"),{target:{value:"atlas"}});
  expect(await screen.findByText("Local changes")).toBeInTheDocument();
  expect(screen.getByText("D:\\Projects\\Atlas\\.aipro\\skills\\jira-weekly-report")).toBeInTheDocument();
  expect(screen.queryByText("C:\\Home\\.aipro\\skills\\jira-weekly-report")).not.toBeInTheDocument();
});

test("a late initial listing cannot hide a project registered while it was loading",async()=>{
  let finishList!:(items:unknown[])=>void;
  vi.mocked(open).mockResolvedValue("D:\\Projects\\Atlas");
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_list_workspaces")return new Promise(resolve=>{finishList=resolve;}) as never;
    if(command==="sds_add_workspace")return {id:"atlas",root:"D:\\Projects\\Atlas"} as never;
    throw new Error(`Unexpected ${command}`);
  });
  render(<WorkspacePage onSelect={vi.fn()} onCancel={vi.fn()}/>);
  fireEvent.click(screen.getByRole("button",{name:"Add project folder"}));
  await waitFor(()=>expect(screen.getByRole("button",{name:"Review requirements"})).toBeEnabled());
  await act(async()=>finishList([]));
  expect(screen.getByLabelText("Registered project")).toHaveValue("atlas");
  expect(screen.getByRole("button",{name:"Review requirements"})).toBeEnabled();
});
