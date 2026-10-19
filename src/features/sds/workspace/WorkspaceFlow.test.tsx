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

beforeEach(()=>{
  vi.mocked(invoke).mockImplementation(async(command,args)=>{
    if(command==="sds_list_workspaces")return [{id:"atlas",root:"D:\\Projects\\Atlas"}] as never;
    if(command==="sds_prepare_install")return {id:"install-atlas",kind:"install",release:release.data,target:{agentKey:"aipro",workspaceId:(args as {request:{target:{workspaceId:string}}}).request.target?.workspaceId??"global",path:"D:\\Projects\\Atlas\\.aipro\\skills\\jira-weekly-report"},mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},requiresMcpAck:false,canExecute:true,conflict:null,expectedFingerprint:null} as never;
    if(command==="sds_remove_workspace")return undefined as never;
    throw new Error(`Unexpected ${command}`);
  });
});

test("install picker defaults to Global without Add project folder button and sends global target",async()=>{
  const onSelect=vi.fn();
  render(<WorkspacePage onSelect={onSelect} onCancel={vi.fn()}/>);
  await screen.findByText(/Each project and the global location/);
  expect(screen.getByRole("radio",{name:"Global"})).toBeChecked();
  expect(screen.queryByRole("button",{name:"Add project folder"})).not.toBeInTheDocument();
  expect(screen.queryByLabelText("Registered project")).not.toBeInTheDocument();

  // Review button is enabled for global even without selecting project
  const reviewBtn=screen.getByRole("button",{name:"Review requirements"});
  expect(reviewBtn).toBeEnabled();
  fireEvent.click(reviewBtn);
  expect(onSelect).toHaveBeenCalledWith({agentKey:"aipro",workspaceId:"global"});
});

test("install picker toggles between Global and Project, requiring a valid project for Project scope",async()=>{
  const onSelect=vi.fn();
  render(<WorkspacePage onSelect={onSelect} onCancel={vi.fn()}/>);
  await screen.findByText(/Each project and the global location/);

  // Switch to Project
  fireEvent.click(screen.getByRole("radio",{name:"Project"}));
  expect(screen.getByRole("button",{name:"Add project folder"})).toBeInTheDocument();
  const select=await screen.findByLabelText("Registered project");
  expect(select).toBeInTheDocument();

  // Initially unselected -> Review disabled
  const reviewBtn=screen.getByRole("button",{name:"Review requirements"});
  expect(reviewBtn).toBeDisabled();

  // Select atlas
  fireEvent.change(select,{target:{value:"atlas"}});
  expect(reviewBtn).toBeEnabled();

  // Switch back to Global
  fireEvent.click(screen.getByRole("radio",{name:"Global"}));
  expect(screen.queryByRole("button",{name:"Add project folder"})).not.toBeInTheDocument();
  expect(screen.queryByLabelText("Registered project")).not.toBeInTheDocument();
  expect(reviewBtn).toBeEnabled();

  fireEvent.click(reviewBtn);
  expect(onSelect).toHaveBeenCalledWith({agentKey:"aipro",workspaceId:"global"});
});

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

test("workspace view selects first project, omits Global option, and does not duplicate path under selector",async()=>{
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_list_workspaces")return [{id:"atlas",root:"D:\\Projects\\Atlas"},{id:"beta",root:"D:\\Projects\\Beta"}] as never;
    if(command==="sds_library")return [{id:"release-key",profileId:"profile-one",release:release.data,cachedAt:"",cacheStatus:"verified",deployments:[{id:"atlas-deployment",target:{agentKey:"aipro",workspaceId:"atlas",path:"D:\\Projects\\Atlas\\.aipro\\skills\\jira-weekly-report"},status:"modified",mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},installedAt:"",reason:null},{id:"global-deployment",target:{agentKey:"aipro",workspaceId:"global",path:"C:\\Home\\.aipro\\skills\\jira-weekly-report"},status:"installed",mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},installedAt:"",reason:null}]}] as never;
    throw new Error(`Unexpected ${command}`);
  });
  render(<WorkspacePage onOperation={vi.fn()}/>);
  const select=await screen.findByLabelText("Registered project") as HTMLSelectElement;
  expect(select.value).toBe("atlas");

  // Global AIPro option must NOT exist in dropdown
  expect(screen.queryByRole("option",{name:"Global AIPro"})).not.toBeInTheDocument();

  // Path is shown in workspace row and deployment table, but NOT duplicated directly under selector
  expect(await screen.findByText("Local changes")).toBeInTheDocument();
  expect(screen.getByText("D:\\Projects\\Atlas\\.aipro\\skills\\jira-weekly-report")).toBeInTheDocument();
  expect(screen.queryByText("C:\\Home\\.aipro\\skills\\jira-weekly-report")).not.toBeInTheDocument();
});

test("workspace view removal of current project selects remaining project and never falls back to global",async()=>{
  vi.mocked(invoke).mockImplementation(async(command,args)=>{
    if(command==="sds_list_workspaces")return [{id:"atlas",root:"D:\\Projects\\Atlas"},{id:"beta",root:"D:\\Projects\\Beta"}] as never;
    if(command==="sds_remove_workspace")return undefined as never;
    if(command==="sds_library")return [] as never;
    throw new Error(`Unexpected ${command} ${JSON.stringify(args)}`);
  });
  render(<WorkspacePage onOperation={vi.fn()}/>);
  const select=await screen.findByLabelText("Registered project") as HTMLSelectElement;
  expect(select.value).toBe("atlas");

  // Remove atlas
  const removeButtons=screen.getAllByRole("button",{name:"Remove registration"});
  fireEvent.click(removeButtons[0]);

  await waitFor(()=>expect(select.value).toBe("beta"));
  expect(select.value).not.toBe("global");
});

test("workspace empty state does not render installed skills or global library",async()=>{
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_list_workspaces")return [] as never;
    if(command==="sds_library")return [{id:"release-key",profileId:"profile-one",release:release.data,cachedAt:"",cacheStatus:"verified",deployments:[{id:"global-deployment",target:{agentKey:"aipro",workspaceId:"global",path:"C:\\Home\\.aipro\\skills\\jira-weekly-report"},status:"installed",mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},installedAt:"",reason:null}]}] as never;
    throw new Error(`Unexpected ${command}`);
  });
  render(<WorkspacePage onOperation={vi.fn()}/>);
  expect(await screen.findByText(/No registered projects yet/)).toBeInTheDocument();
  expect(screen.queryByRole("heading",{name:"Installed AIPro skills"})).not.toBeInTheDocument();
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
  // In onSelect mode, click Project first to show Add project folder
  fireEvent.click(screen.getByRole("radio",{name:"Project"}));
  fireEvent.click(screen.getByRole("button",{name:"Add project folder"}));
  await waitFor(()=>expect(screen.getByRole("button",{name:"Review requirements"})).toBeEnabled());
  await act(async()=>finishList([]));
  expect(screen.getByLabelText("Registered project")).toHaveValue("atlas");
  expect(screen.getByRole("button",{name:"Review requirements"})).toBeEnabled();
});
