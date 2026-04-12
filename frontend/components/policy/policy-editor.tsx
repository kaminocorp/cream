import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ProfileForm } from "./profile-form";
import { RuleList } from "./rule-list";
import { AgentProfile, PolicyRule } from "@/lib/types";

interface PolicyEditorProps {
  agentId: string;
  profile: AgentProfile;
  rules: PolicyRule[];
}

/**
 * Policy editor — two tabs:
 *
 * 1. **Profile** — editable spending limits, categories, rails, geo
 *    restrictions, escalation threshold. Saves via PUT /v1/agents/{id}/policy.
 *
 * 2. **Rules** — read-only display of the policy rule tree. Each rule
 *    expands to show its recursive PolicyCondition tree. Rule creation,
 *    editing, and deletion require backend endpoints not yet implemented
 *    (deferred to a future sub-phase).
 */
export function PolicyEditor({ agentId, profile, rules }: PolicyEditorProps) {
  return (
    <Tabs defaultValue="profile">
      <TabsList>
        <TabsTrigger value="profile">Profile Settings</TabsTrigger>
        <TabsTrigger value="rules">
          Rules ({rules.length})
        </TabsTrigger>
      </TabsList>

      <TabsContent value="profile" className="mt-4">
        <ProfileForm agentId={agentId} profile={profile} />
      </TabsContent>

      <TabsContent value="rules" className="mt-4">
        <RuleList rules={rules} />
        <p className="mt-4 text-xs text-zinc-400">
          Rule editing (create, update, delete, reorder) requires backend
          endpoints not yet available. Use{" "}
          <code className="rounded bg-zinc-100 px-1">
            PUT /v1/agents/{"{id}"}/policy
          </code>{" "}
          for profile-level settings above, or manage rules directly via
          the database.
        </p>
      </TabsContent>
    </Tabs>
  );
}
