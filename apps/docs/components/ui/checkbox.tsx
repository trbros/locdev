"use client";

import { Checkbox as CheckboxPrimitive } from "@base-ui/react/checkbox";
import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

export function Checkbox({
	className,
	...props
}: CheckboxPrimitive.Root.Props) {
	return (
		<CheckboxPrimitive.Root
			data-slot="checkbox"
			className={cn("site-checkbox", className)}
			{...props}
		>
			<CheckboxPrimitive.Indicator
				data-slot="checkbox-indicator"
				className="site-checkbox-indicator"
			>
				<Check size={12} />
			</CheckboxPrimitive.Indicator>
		</CheckboxPrimitive.Root>
	);
}
