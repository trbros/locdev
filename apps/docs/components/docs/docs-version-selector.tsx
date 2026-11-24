"use client";

import { ChoiceSelect } from "@/components/ui/choice-select";

export function DocsVersionSelector({
	version,
	label,
	current,
}: {
	version: string;
	label: string;
	current: string;
}) {
	return (
		<div className="docs-version-selector">
			<p>{label}</p>
			<ChoiceSelect
				label={label}
				value={version}
				onChange={() => {
					/* Only the current documentation version is available. */
				}}
				options={[{ value: version, label: `${version} · ${current}` }]}
			/>
		</div>
	);
}
