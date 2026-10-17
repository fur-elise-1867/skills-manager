import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import CatalogPage from "./CatalogPage";
import { SdsSessionContext } from "../session/context";
import type { SdsSessionState } from "../session/context";
import taxonomy from "../../../../contracts/sds-desktop/v1/fixtures/taxonomy.json";
import catalog from "../../../../contracts/sds-desktop/v1/fixtures/catalog.json";
import detail from "../../../../contracts/sds-desktop/v1/fixtures/detail.json";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const account: SdsSessionState = { connection: {id:"profile-one",baseUrl:"https://store.example.invalid/skm",environment:"internal"},session:{status:"authenticated",user:{id:"1001",displayName:"Example Employee"}},generation:1,error:"",signedIn:vi.fn(),connectionSaved:vi.fn(),logout:vi.fn(),refreshStatus:vi.fn() };
function tree(state = account) { return <SdsSessionContext.Provider value={state}><CatalogPage /></SdsSessionContext.Provider>; }
beforeEach(() => {
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === "sds_catalog_taxonomy") return taxonomy.data as never;
    if (command === "sds_catalog_search") return catalog.data as never;
    if (command === "sds_skill_detail") {
      const { latestRelease,storePageUrl,...summary } = detail.data;
      return { summary:{...summary,latestRelease:{id:latestRelease.id,sequence:latestRelease.sequence,version:latestRelease.version}},latestRelease,storePageUrl } as never;
    }
    throw new Error(`Unexpected command ${command}`);
  });
});

test("browses the server catalog and detail without inventing compatibility or loading remote Markdown", async () => {
  render(tree());
  await screen.findByRole("button",{name:"View Jira Weekly Report"});
  expect(screen.getByText("Compatibility not declared")).toBeInTheDocument();
  expect(screen.queryByText("Installed")).not.toBeInTheDocument();
  expect(screen.getByRole("button",{name:"Local Install"})).toBeDisabled();
  fireEvent.click(screen.getByRole("button",{name:"View Jira Weekly Report"}));
  await screen.findByRole("heading",{name:"Jira Weekly Report"});
  expect(await screen.findByText("jira-mcp")).toBeInTheDocument();
  expect(document.querySelector("img")).toBeNull();
  expect(document.querySelector('a[href^="javascript:"]')).toBeNull();
  expect(screen.getByText("Add blocked-task summary.")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Back to results"}));
  expect(screen.getByRole("button",{name:"View Jira Weekly Report"})).toBeInTheDocument();
});

test("combines search and Source, resets pagination, and retains filters after returning from detail", async () => {
  vi.mocked(invoke).mockImplementation(async (command,args) => {
    if(command === "sds_catalog_taxonomy") return taxonomy.data as never;
    if(command === "sds_skill_detail") return {summary:catalog.data.items[0],latestRelease:detail.data.latestRelease,storePageUrl:null} as never;
    const query = (args as {query:{page:number;q:string}}).query;
    return {...catalog.data,page:query.page,totalItems:50} as never;
  });
  render(tree());
  await screen.findByRole("button",{name:"View Jira Weekly Report"});
  fireEvent.click(screen.getByRole("button",{name:"Next page"}));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("sds_catalog_search",{query:expect.objectContaining({page:2})}));
  fireEvent.change(screen.getByLabelText("Search skills"),{target:{value:"jira & report"}});
  fireEvent.submit(screen.getByRole("search"));
  fireEvent.click(screen.getByRole("button",{name:"Quality"}));
  await waitFor(() => expect(invoke).toHaveBeenLastCalledWith("sds_catalog_search",{query:{q:"jira & report",mode:"all_time",sourceId:"quality",page:1,pageSize:20}}));
  await screen.findByRole("button",{name:"View Jira Weekly Report"});
  fireEvent.click(screen.getByRole("button",{name:"View Jira Weekly Report"}));
  await screen.findByText("jira-mcp"); fireEvent.click(screen.getByRole("button",{name:"Back to results"}));
  expect(screen.getByLabelText("Search skills")).toHaveValue("jira & report");
  expect(screen.getByRole("button",{name:"Quality"})).toHaveAttribute("aria-pressed","true");
});

test("does not substitute catalog compatibility when the release has no declaration",async()=>{
  vi.mocked(invoke).mockImplementation(async(command)=>{
    if(command === "sds_catalog_taxonomy") return taxonomy.data as never;
    if(command === "sds_catalog_search") return catalog.data as never;
    return {summary:{...catalog.data.items[0],compatibleAgents:["aipro"]},latestRelease:detail.data.latestRelease,storePageUrl:null} as never;
  });
  render(tree());
  fireEvent.click(await screen.findByRole("button",{name:"View Jira Weekly Report"}));
  await screen.findByText("jira-mcp");
  expect(screen.getByText("Compatibility not declared")).toBeInTheDocument();
  expect(screen.queryByText("Declared compatibility: AIPro")).not.toBeInTheDocument();
});

test("discards a slow result from a previous query and from a previous account", async () => {
  const pending: ((value: unknown) => void)[] = [];
  vi.mocked(invoke).mockImplementation(async (command) => {
    if(command === "sds_catalog_taxonomy") return taxonomy.data as never;
    return new Promise((resolve)=>pending.push(resolve)) as never;
  });
  const view = render(tree());
  await waitFor(()=>expect(pending).toHaveLength(1));
  fireEvent.change(screen.getByLabelText("Search skills"),{target:{value:"new query"}});
  fireEvent.submit(screen.getByRole("search"));
  await waitFor(()=>expect(pending).toHaveLength(2));
  await act(async()=>pending[1]({...catalog.data,items:[{...catalog.data.items[0],name:"Current result"}]}));
  await screen.findByRole("button",{name:"View Current result"});
  await act(async()=>pending[0]({...catalog.data,items:[{...catalog.data.items[0],name:"Stale query result"}]}));
  expect(screen.queryByText("Stale query result")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Search"}));
  await waitFor(()=>expect(pending).toHaveLength(3));
  view.rerender(tree({...account,generation:2,session:{status:"authenticated",user:{id:"1002",displayName:"Another employee"}}}));
  await waitFor(()=>expect(pending).toHaveLength(4));
  expect(screen.queryByText("Current result")).not.toBeInTheDocument();
  await act(async()=>pending[2]({...catalog.data,items:[{...catalog.data.items[0],name:"Previous account result"}]}));
  expect(screen.queryByText("Previous account result")).not.toBeInTheDocument();
  await act(async()=>pending[3]({...catalog.data,items:[],totalItems:0}));
  await screen.findByRole("heading",{name:"No skills found"});
});

test.each([
  ["CATALOG_MODE_UNAVAILABLE","This catalog mode is not available. Choose another mode."],
  ["SESSION_EXPIRED","Your session has expired. Sign in again to access Skills Store."],
  ["STORE_UNAVAILABLE","Skills Store is unavailable. Check your connection and try again."],
])("reports %s without showing a false empty state",async(code,message)=>{
  vi.mocked(invoke).mockImplementation(async(command)=>{
    if(command === "sds_catalog_taxonomy") return taxonomy.data as never;
    throw {code,message:"synthetic-access-one"};
  });
  render(tree());
  expect(await screen.findByRole("alert")).toHaveTextContent(message);
  expect(screen.queryByRole("heading",{name:"No skills found"})).not.toBeInTheDocument();
  expect(screen.queryByText("synthetic-access-one")).not.toBeInTheDocument();
});
