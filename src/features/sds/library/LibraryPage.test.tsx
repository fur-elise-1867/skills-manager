import {fireEvent,render,screen} from "@testing-library/react";
import {beforeEach,expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import {writeText} from "@tauri-apps/plugin-clipboard-manager";
import LibraryPage from "./LibraryPage";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";

vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/plugin-clipboard-manager",()=>({writeText:vi.fn(async()=>{})}));
const item={id:"a".repeat(64),profileId:"profile-one",release:release.data,cachedAt:"2026-10-17T00:00:00Z",cacheStatus:"verified",deployments:[{id:"deployment-one",target:{agentKey:"aipro",workspaceId:"global",path:"C:\\Synthetic\\.aipro\\skills\\jira-weekly-report"},status:"installed",mcp:{state:"missing",missing:["jira-mcp"],reason:null,checkedAt:"2026-10-17T00:00:00Z"},installedAt:"2026-10-17T00:00:00Z",reason:null}]};
beforeEach(()=>{vi.mocked(invoke).mockImplementation(async command=>{
    if(command==="sds_library")return [item] as never;
    if(command==="sds_library_preview")return "# Local skill\n![remote](https://example.invalid/tracker.png)\n[External](https://example.invalid/)" as never;
    if(command==="sds_reveal_deployment")return undefined as never;
    throw new Error(`Unexpected command ${command}`);
});});

test("shows independent cache and target status locally, searches paths and does not expose deferred preview or install controls",async()=>{
    render(<LibraryPage onInstall={vi.fn()} onCachedInstall={vi.fn()}/>);
    await screen.findByRole("heading",{name:"jira-weekly-report"});
    expect(screen.getByText(item.deployments[0].target.path)).toBeInTheDocument();
    expect(screen.getByText("Installed",{selector:"span"})).toBeInTheDocument();
    expect(screen.getByText(/MCP at installation: Missing/)).toBeInTheDocument();

    expect(screen.queryByRole("button",{name:/Preview/i})).not.toBeInTheDocument();
    expect(screen.queryByRole("button",{name:/Install skill/i})).not.toBeInTheDocument();
    expect(screen.queryByRole("button",{name:/Use cached copy/i})).not.toBeInTheDocument();
    expect(screen.queryByRole("button",{name:/Refresh Library/i})).not.toBeInTheDocument();
    expect(screen.queryByRole("button",{name:/^All$/i})).not.toBeInTheDocument();
    expect(screen.queryByRole("button",{name:/^Installed$/i})).not.toBeInTheDocument();
    expect(screen.queryByRole("button",{name:/^Available$/i})).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole("searchbox",{name:"Search Library"}),{target:{value:"not-in-library"}});
    expect(screen.getByText("No matching releases.")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("searchbox",{name:"Search Library"}),{target:{value:"Synthetic"}});
    expect(screen.getByText("jira-weekly-report")).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.map(([command])=>command)).toEqual(["sds_library"]);
});

test("reveals only the selected backend deployment ID and copies its displayed path on click",async()=>{
    render(<LibraryPage/>);
    await screen.findByRole("button",{name:"Reveal folder"});
    expect(invoke).not.toHaveBeenCalledWith("sds_reveal_deployment",expect.anything());
    fireEvent.click(screen.getByRole("button",{name:"Reveal folder"}));
    expect(invoke).toHaveBeenCalledWith("sds_reveal_deployment",{deploymentId:item.deployments[0].id});
    fireEvent.click(screen.getByRole("button",{name:"Copy path"}));
    expect(await screen.findByRole("status")).toHaveTextContent("Destination path copied");
    expect(writeText).toHaveBeenCalledWith(item.deployments[0].target.path);
});

test("an unreadable Library is an error rather than an empty result and supports retry",async()=>{
    vi.mocked(invoke).mockRejectedValueOnce({code:"REPOSITORY_UNAVAILABLE",message:"synthetic-secret"});
    render(<LibraryPage/>);
    expect(await screen.findByRole("alert")).toHaveTextContent("Library could not be read");
    expect(screen.queryByText("No downloaded releases yet.")).not.toBeInTheDocument();
    expect(screen.queryByText("synthetic-secret")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button",{name:"Try again"}));
    expect(await screen.findByRole("heading",{name:"jira-weekly-report"})).toBeInTheDocument();
});

test("cache problems do not erase modified or missing target records and allows deleting the downloaded copy",async()=>{
    vi.mocked(invoke).mockResolvedValue([{...item,cacheStatus:"corrupt",deployments:[{...item.deployments[0],status:"modified",reason:"Local contents have changed."},{...item.deployments[0],id:"missing",status:"missing",target:{...item.deployments[0].target,path:"C:\\Synthetic\\another-target"},reason:"The installed folder is missing."}]}] as never);
    render(<LibraryPage/>);
    expect(await screen.findByText("Cache needs attention")).toBeInTheDocument();
    expect(screen.getByText("Local changes")).toBeInTheDocument();
    expect(screen.getByText("Missing folder")).toBeInTheDocument();
    expect(screen.getByRole("button",{name:"Delete from Library"})).toBeEnabled();
    expect(screen.queryByRole("button",{name:/Preview/i})).not.toBeInTheDocument();
});
