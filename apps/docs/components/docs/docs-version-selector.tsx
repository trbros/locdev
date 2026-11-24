"use client";

import { ChoiceSelect } from "@/components/ui/choice-select";
import { useRouter } from "@/i18n/navigation";

/** The documentation follows the current version only. The earlier ones,
 *  from the changelog, open their entry there: what changed in each. */
export function DocsVersionSelector({
	version,
	versions,
	label,
}: {
	version: string;
	versions: readonly { value: string; label: string }[];
	label: string;
}) {
	const router = useRouter();
	return (
		<div className="docs-version-selector">
			<p>{label}</p>
			<ChoiceSelect
				label={label}
				value={version}
				onChange={(value) => {
					if (value !== version) router.push(`/changelog#${value}`);
				}}
				options={versions}
			/>
		</div>
	);
}
