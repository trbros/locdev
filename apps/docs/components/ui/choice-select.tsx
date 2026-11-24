"use client";
import { Select } from "@base-ui/react/select";
import { Check, ChevronDown } from "lucide-react";
export function ChoiceSelect({
	label,
	value,
	onChange,
	options,
	id,
}: {
	label: string;
	value: string;
	onChange: (value: string) => void;
	options: readonly { value: string; label: string }[];
	id?: string;
}) {
	return (
		<Select.Root
			value={value}
			onValueChange={(value) => {
				if (value !== null) onChange(value);
			}}
			items={options}
		>
			<Select.Trigger id={id} aria-label={label} className="choice-trigger">
				<Select.Value />
				<Select.Icon>
					<ChevronDown size={15} />
				</Select.Icon>
			</Select.Trigger>
			<Select.Portal>
				<Select.Positioner
					className="choice-positioner"
					sideOffset={6}
					align="start"
				>
					<Select.Popup className="choice-popup">
						<Select.List>
							{options.map((option) => (
								<Select.Item
									key={option.value}
									value={option.value}
									className="choice-item"
								>
									<Select.ItemText>{option.label}</Select.ItemText>
									<Select.ItemIndicator>
										<Check size={14} />
									</Select.ItemIndicator>
								</Select.Item>
							))}
						</Select.List>
					</Select.Popup>
				</Select.Positioner>
			</Select.Portal>
		</Select.Root>
	);
}
