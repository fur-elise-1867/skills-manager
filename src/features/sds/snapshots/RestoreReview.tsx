import InstallReview from "../install/InstallReview";
import type {ComponentProps} from "react";

export default function RestoreReview(props:ComponentProps<typeof InstallReview>) {
  return <InstallReview {...props}/>;
}
