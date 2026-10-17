import {fireEvent,render,screen,waitFor} from "@testing-library/react";
import {expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import SdsApp from "../SdsApp";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
test("Retry connection makes a protected metadata request before refreshing remembered session state",async()=>{
  let recovered=false,retryAllowed=false;
  vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="runtime_profile")return {kind:"sds",environment:"internal",fixtureMode:false} as never;
    if(command==="sds_get_connection")return {id:"profile-one",baseUrl:"https://store.example.invalid/skm",environment:"internal"} as never;
    if(command==="sds_session_status")return {status:recovered?"authenticated":"offline",user:{id:"u",displayName:"Employee"}} as never;
    if(command==="sds_catalog_taxonomy"){if(!retryAllowed)throw {code:"STORE_UNAVAILABLE"};recovered=true;return {sources:[],catalogModes:[]} as never;}
    if(command==="sds_catalog_search")throw {code:"STORE_UNAVAILABLE"};
    if(command==="sds_check_updates")return {checkedAt:"now",items:[]} as never;
    if(command==="sds_library")return [] as never;
    throw new Error(`Unexpected ${command}`);
  });
  render(<SdsApp/>);
  fireEvent.click(await screen.findByRole("button",{name:"Library"}));
  await screen.findByRole("button",{name:"Retry connection"});
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith("sds_catalog_taxonomy"));
  retryAllowed=true;
  const before=vi.mocked(invoke).mock.calls.filter(([name])=>name==="sds_catalog_taxonomy").length;
  fireEvent.click(screen.getByRole("button",{name:"Retry connection"}));
  await waitFor(()=>expect(vi.mocked(invoke).mock.calls.filter(([name])=>name==="sds_catalog_taxonomy").length).toBeGreaterThan(before));
  await waitFor(()=>expect(screen.getByRole("button",{name:"Check for updates"})).toBeEnabled());
});
