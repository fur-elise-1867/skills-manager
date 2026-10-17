import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import InstallReview from "./InstallReview";
import type { PreparedOperation, Release } from "../contracts";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";

const operation: PreparedOperation = {id:"operation-one",kind:"install",release:release.data as Release,target:{agentKey:"aipro",workspaceId:"global",path:"C:\\Fixture AIPro\\skills\\jira-weekly-report"},expectedFingerprint:null,mcp:{state:"missing",missing:["jira-mcp"],reason:null,checkedAt:"2026-10-17T00:00:00Z"},requiresMcpAck:true,canExecute:true,conflict:null};

test("shows the exact target and allows an explicit MCP warning acknowledgment bound to the operation",()=>{
  const confirm = vi.fn(); const cancel = vi.fn(); const check = vi.fn();
  render(<InstallReview operation={operation} pending={false} onConfirm={confirm} onCancel={cancel} onCheckAgain={check} />);
  expect(screen.getByRole("dialog",{name:"Install skill"})).toBeInTheDocument();
  expect(screen.getByText(operation.target.path)).toBeInTheDocument();
  expect(screen.getByText("jira-mcp")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Check again"})); expect(check).toHaveBeenCalledOnce();
  fireEvent.click(screen.getByRole("button",{name:"Install anyway"})); expect(confirm).toHaveBeenCalledWith("operation-one",true);
  fireEvent.keyDown(screen.getByRole("dialog"),{key:"Escape"}); expect(cancel).toHaveBeenCalledOnce();
});

test("a fresh MCP report changes the review but never claims the server is operational",()=>{
  const confirm = vi.fn(); const props = {onConfirm:confirm,onCancel:vi.fn(),onCheckAgain:vi.fn(),pending:false};
  const view = render(<InstallReview operation={operation} {...props} />);
  view.rerender(<InstallReview operation={{...operation,mcp:{...operation.mcp,state:"configured",missing:[]},requiresMcpAck:false}} {...props} />);
  expect(screen.getByText("Configuration found")).toBeInTheDocument();
  expect(screen.getByText(/does not verify that MCP servers are running/i)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Install skill"})); expect(confirm).toHaveBeenCalledWith("operation-one",false);
  view.rerender(<InstallReview operation={{...operation,canExecute:false,conflict:"The destination contains unmanaged files."}} {...props} />);
  expect(screen.getByRole("alert")).toHaveTextContent("unmanaged files");
  expect(screen.getByRole("button",{name:"Install anyway"})).toBeDisabled();
});
