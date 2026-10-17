import {act,fireEvent,render,screen,waitFor} from "@testing-library/react";
import {beforeEach,expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import SdsShell from "../SdsShell";
import {SdsSessionContext} from "../session/context";
import type {SdsSessionState} from "../session/context";
import type {UpdateCheck} from "../contracts";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
const account:SdsSessionState={session:{status:"authenticated",user:{id:"account-one",displayName:"Employee"}},connection:{id:"profile-one",baseUrl:"https://store.example.invalid/skm",environment:"internal"},generation:1,error:"",signedIn:vi.fn(),connectionSaved:vi.fn(),logout:vi.fn(),refreshStatus:vi.fn()};
const item={id:"release-key",profileId:"profile-one",release:release.data,cachedAt:"",cacheStatus:"verified",deployments:[]};
const updates:UpdateCheck={checkedAt:"now",items:[{skillId:release.data.skillId,currentReleaseId:release.data.id,status:"update_available",latestRelease:{id:"new-release",version:"1.3.0",sequence:4}}]};
beforeEach(()=>{vi.mocked(invoke).mockImplementation(async command=>{
  if(command==="sds_library")return [item] as never;
  if(command==="sds_check_updates")return updates as never;
  if(command==="sds_catalog_taxonomy")return {sources:[],catalogModes:[]} as never;
  if(command==="sds_catalog_search")return {items:[],page:1,pageSize:20,totalItems:0} as never;
  throw new Error(`Unexpected ${command}`);
});});
const shell=(context=account)=><SdsSessionContext.Provider value={context}><SdsShell profile={{kind:"sds",environment:"internal",fixtureMode:false}}/></SdsSessionContext.Provider>;

test("startup metadata check runs once and manual check never downloads packages",async()=>{
  render(shell());
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith("sds_check_updates"));
  fireEvent.click(screen.getByRole("button",{name:"Library"}));
  await screen.findByRole("button",{name:"Review update"});
  expect(vi.mocked(invoke).mock.calls.filter(([name])=>name==="sds_check_updates")).toHaveLength(1);
  fireEvent.click(screen.getByRole("button",{name:"Check for updates"}));
  await waitFor(()=>expect(vi.mocked(invoke).mock.calls.filter(([name])=>name==="sds_check_updates")).toHaveLength(2));
  expect(vi.mocked(invoke).mock.calls.some(([name])=>name==="sds_prepare_update" || name==="sds_execute")).toBe(false);
});

test("late metadata and an open selection from the previous account never enter a new account",async()=>{
  let resolveOld!:(value:UpdateCheck)=>void;
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_check_updates")return new Promise(resolve=>{resolveOld=resolve;}) as never;
    if(command==="sds_library")return [item] as never;
    if(command==="sds_catalog_taxonomy")return {sources:[],catalogModes:[]} as never;
    if(command==="sds_catalog_search")return {items:[],page:1,pageSize:20,totalItems:0} as never;
    throw new Error(`Unexpected ${command}`);
  });
  const view=render(shell());
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith("sds_check_updates"));
  fireEvent.click(screen.getByRole("button",{name:"Library"}));
  view.rerender(shell({...account,generation:2,session:{status:"signedOut",user:null}}));
  await act(async()=>resolveOld(updates));
  expect(screen.queryByRole("button",{name:"Review update"})).not.toBeInTheDocument();
  expect(screen.getByRole("button",{name:"Check for updates"})).toBeDisabled();
});
