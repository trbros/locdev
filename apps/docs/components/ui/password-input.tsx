"use client";
import { Eye, EyeOff } from "lucide-react";
import { type ComponentProps, useState } from "react";
import { Button } from "./button";
import { Input } from "./input";
export function PasswordInput({
	showLabel,
	hideLabel,
	...props
}: Omit<ComponentProps<typeof Input>, "type"> & {
	showLabel: string;
	hideLabel: string;
}) {
	const [visible, setVisible] = useState(false);
	return (
		<div className="site-password-input">
			<Input {...props} type={visible ? "text" : "password"} />
			<Button
				type="button"
				variant="ghost"
				size="icon"
				aria-label={visible ? hideLabel : showLabel}
				aria-pressed={visible}
				onClick={() => setVisible(!visible)}
			>
				{visible ? <EyeOff size={16} /> : <Eye size={16} />}
			</Button>
		</div>
	);
}
