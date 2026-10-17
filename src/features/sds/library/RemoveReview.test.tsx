import {fireEvent,render,screen} from "@testing-library/react";
import {expect,test,vi} from "vitest";
import RemoveReview from "./RemoveReview";
import type {PreparedOperation,Release} from "../contracts";
import release from "../../../../contracts/sds-desktop/v1/fixtures/release.json";
const plan:PreparedOperation={id:"remove-one",kind:"remove",release:release.data as Release,target:{agentKey:"aipro",workspaceId:"global",path:"C:\\Synthetic\\skills\\jira-weekly-report"},mcp:{state:"missing",missing:["jira-mcp"],reason:null,checkedAt:""},expectedFingerprint:"a".repeat(64),requiresMcpAck:false,canExecute:true,conflict:null};
test("reviews the exact scoped removal and keeps Library and other destinations",()=>{
  const confirm=vi.fn(),cancel=vi.fn();render(<RemoveReview operation={plan} pending={false} onConfirm={confirm} onCancel={cancel}/>);
  expect(screen.getByRole("dialog",{name:"Remove skill"})).toBeInTheDocument();
  expect(screen.getByText(plan.target.path)).toBeInTheDocument();
  expect(screen.getByText(/Library copy and other destinations will be kept/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Remove from target"}));expect(confirm).toHaveBeenCalledWith("remove-one",false);
  fireEvent.keyDown(screen.getByRole("dialog"),{key:"Escape"});expect(cancel).toHaveBeenCalled();
});
test("local changes cannot be force deleted and missing target retires metadata only",()=>{
  const props={pending:false,onConfirm:vi.fn(),onCancel:vi.fn()};
  const view=render(<RemoveReview {...props} operation={{...plan,canExecute:false,conflict:"Local files have changed."}}/>);
  expect(screen.getByRole("alert")).toHaveTextContent("Local files have changed");
  expect(screen.getByRole("button",{name:"Remove from target"})).toBeDisabled();
  view.rerender(<RemoveReview {...props} operation={{...plan,expectedFingerprint:null}}/>);
  expect(screen.getByRole("button",{name:"Remove missing record"})).toBeEnabled();
});
