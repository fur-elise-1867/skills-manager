import {fireEvent,render,screen,waitFor} from "@testing-library/react";
import {beforeEach,expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import LibraryPage from "../library/LibraryPage";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";
import OperationFlow from "../install/OperationFlow";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/event",()=>({listen:vi.fn(async()=>vi.fn())}));
const item={id:"release-key",profileId:"profile-one",release:release.data,cachedAt:"2026-10-17T00:00:00Z",cacheStatus:"verified",deployments:[]};
beforeEach(()=>{vi.mocked(invoke).mockImplementation(async command=>{
  if(command==="sds_library")return [item] as never;
  throw new Error(`Unexpected ${command}`);
});});

test("offline cached install is explicit, shows provenance and never invokes ordinary Store install",async()=>{
  const install=vi.fn(),cached=vi.fn();render(<LibraryPage onInstall={install} onCachedInstall={cached} session={{status:"offline",user:{id:"account-one",displayName:"Employee"}}} profileId="profile-one"/>);
  fireEvent.click(await screen.findByRole("button",{name:"Use cached copy"}));
  expect(cached).toHaveBeenCalledWith("release-key");expect(install).not.toHaveBeenCalled();
  expect(screen.getByText(/Current Store permissions are unknown/)).toBeInTheDocument();
  expect(screen.getByText(/2026-10-17T00:00:00Z/)).toBeInTheDocument();
  expect(invoke).not.toHaveBeenCalledWith("sds_prepare_install",expect.anything());
});

test.each(["expired","signedOut"] as const)("%s cannot deploy cached releases but keeps local maintenance",async status=>{
  render(<LibraryPage onInstall={vi.fn()} onCachedInstall={vi.fn()} session={{status,user:null}} profileId="profile-one"/>);
  await screen.findByRole("heading",{name:"jira-weekly-report"});
  expect(screen.getByRole("button",{name:"Use cached copy"})).toBeDisabled();
  expect(screen.getByText(/Sign in to deploy/)).toBeInTheDocument();
});

test("cache deletion has a separate reviewed command and does not remove a target",async()=>{
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_library")return [item] as never;
    if(command==="sds_prepare_cache_delete")return {id:"delete-plan",releaseKey:"release-key",expectedFingerprint:"cache",runtimeEpoch:"epoch"} as never;
    if(command==="sds_execute_cache_delete")return {operationId:"delete-plan",state:"completed",stage:"completed",error:null} as never;
    throw new Error(`Unexpected ${command}`);
  });
  render(<LibraryPage/>);
  fireEvent.click(await screen.findByRole("button",{name:"Delete from Library"}));
  await screen.findByRole("dialog",{name:"Delete downloaded copy"});
  expect(invoke).not.toHaveBeenCalledWith("sds_execute_cache_delete",expect.anything());
  fireEvent.click(await screen.findByRole("button",{name:"Delete cached files"}));
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith("sds_execute_cache_delete",{planId:"delete-plan"}));
  expect(vi.mocked(invoke).mock.calls.some(([name])=>name==="sds_prepare_remove")).toBe(false);
});

test("native cache grant denial explains online authorization without falling back to Store install",async()=>{
  vi.mocked(invoke).mockRejectedValue({code:"CACHE_NOT_AUTHORIZED",message:"native internal details"});
  render(<OperationFlow request={{kind:"cached",releaseKey:"release-key",target:{agentKey:"aipro",workspaceId:"global"}}} onClose={vi.fn()} onCompleted={vi.fn()}/>);
  expect(await screen.findByRole("alert")).toHaveTextContent(/Reconnect.*authorize.*online/i);
  expect(invoke).toHaveBeenCalledWith("sds_prepare_cached_install",{releaseKey:"release-key",target:{agentKey:"aipro",workspaceId:"global"}});
  expect(vi.mocked(invoke).mock.calls.some(([name])=>name==="sds_prepare_install")).toBe(false);
});
