import { ActionsView } from "@/pages/actions";
import { CliView } from "@/pages/cli";
import { DatabaseView } from "@/pages/database";
import { DebugView } from "@/pages/debug";
import { DoctorView } from "@/pages/doctor";
import { InstallersView, NewProjectView } from "@/pages/new-project";
import { MailboxView } from "@/pages/mailbox";
import { ProcessesView } from "@/pages/processes";
import { TunnelsView } from "@/pages/tunnels";
import { useUi } from "@/lib/ui-state";

/** Tools: each tool is its own page, listed under Tools in the sidebar. */
export function ToolsView() {
  const { sub } = useUi();
  switch (sub ?? "processes") {
    case "new-project":
      return <NewProjectView />;
    case "installers":
      return <InstallersView />;
    case "debug":
      return <DebugView />;
    case "database":
      return <DatabaseView />;
    case "mailbox":
      return <MailboxView />;
    case "tunnels":
      return <TunnelsView />;
    case "actions":
      return <ActionsView />;
    case "cli":
      return <CliView />;
    case "doctor":
      return <DoctorView />;
    default:
      return <ProcessesView />;
  }
}
