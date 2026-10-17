import {fireEvent,render,screen,waitFor,within} from "@testing-library/react";
import {beforeEach,expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import LibraryPage from "./LibraryPage";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/plugin-clipboard-manager",()=>({writeText:vi.fn(async()=>{})}));
const deployment={id:"atlas",target:{agentKey:"aipro",workspaceId:"atlas",path:"D:\\Atlas\\.aipro\\skills\\jira-weekly-report"},status:"modified",expectedFingerprint:"edited-fingerprint",mcp:{state:"notRequired",missing:[],reason:null,checkedAt:""},installedAt:"2026-10-17T00:00:00Z",reason:null};
const item={id:"release-key",profileId:"profile-one",release:release.data,cachedAt:"2026-10-17T00:00:00Z",cacheStatus:"verified",deployments:[deployment]};
beforeEach(()=>{vi.mocked(invoke).mockImplementation(async command=>{
  if(command==="sds_library")return [item] as never;
  if(command==="sds_capture_snapshot")return {id:"snapshot-one"} as never;
  throw new Error(`Unexpected ${command}`);
});});

test("modified removal requires explicit acknowledgement and defaults to verified backup",async()=>{
  const remove=vi.fn();render(<LibraryPage onRemove={remove}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Remove from target"}));
  const dialog=await screen.findByRole("dialog",{name:"Protect local changes"});
  expect(remove).not.toHaveBeenCalled();
  expect(within(dialog).getByLabelText("Save a local backup before removing")).toBeChecked();
  expect(within(dialog).getByRole("button",{name:"Review removal"})).toBeDisabled();
  fireEvent.click(within(dialog).getByLabelText("I understand this removes the local changes from this target"));
  fireEvent.click(within(dialog).getByRole("button",{name:"Review removal"}));
  expect(remove).toHaveBeenCalledWith("atlas",{backup:true,acknowledgeLocalEdits:true});
});

test("manual snapshot passes the current observed fingerprint and reports completion",async()=>{
  render(<LibraryPage/>);
  fireEvent.click(await screen.findByRole("button",{name:"Create snapshot"}));
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith("sds_capture_snapshot",{deploymentId:"atlas",expectedFingerprint:"edited-fingerprint",reason:"manual"}));
  expect(await screen.findByText("Local snapshot saved.")).toBeInTheDocument();
});
