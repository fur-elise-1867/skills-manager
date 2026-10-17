import {fireEvent,render,screen} from "@testing-library/react";
import {expect,test,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import ActivitySettings from "./ActivitySettings";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
test("opens only the backend logs location on user click and keeps raw errors private",async()=>{
  vi.mocked(invoke).mockRejectedValue({message:"synthetic-secret"});
  render(<ActivitySettings/>);
  expect(invoke).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button",{name:"Reveal logs folder"}));
  expect(invoke).toHaveBeenCalledWith("sds_reveal_logs");
  expect(await screen.findByRole("alert")).toHaveTextContent("could not be shown");
  expect(screen.queryByText("synthetic-secret")).not.toBeInTheDocument();
});
