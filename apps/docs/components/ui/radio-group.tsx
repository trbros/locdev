"use client";
import { Radio } from "@base-ui/react/radio";
import { RadioGroup as RadioGroupPrimitive } from "@base-ui/react/radio-group";
import { cn } from "@/lib/utils";
export function RadioGroup(props: RadioGroupPrimitive.Props<string>) {
	return <RadioGroupPrimitive data-slot="radio-group" {...props} />;
}
export function RadioGroupItem({
	className,
	children,
	...props
}: Radio.Root.Props<string>) {
	return (
		<Radio.Root
			data-slot="radio-group-item"
			className={cn("site-radio", className)}
			{...props}
		>
			{children}
			<span className="site-radio-circle">
				<Radio.Indicator className="site-radio-dot" />
			</span>
		</Radio.Root>
	);
}
