"use client";

import { Input as InputPrimitive } from "@base-ui/react/input";
import { cn } from "@/lib/utils";

export function Input({ className, ...props }: InputPrimitive.Props) {
	return (
		<InputPrimitive
			data-slot="input"
			className={cn(
				"site-input min-w-0 outline-none disabled:opacity-50",
				className,
			)}
			{...props}
		/>
	);
}
