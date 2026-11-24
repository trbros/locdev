"use client";

import { Slider as SliderPrimitive } from "@base-ui/react/slider";
import { cn } from "@/lib/utils";

/** shadcn/base-nova composition, using the project's installed Base UI primitives. */
export function Slider({
	className,
	thumbLabel,
	...props
}: SliderPrimitive.Root.Props<number[]> & { thumbLabel: string }) {
	return (
		<SliderPrimitive.Root
			data-slot="slider"
			className={cn("site-slider", className)}
			{...props}
		>
			<SliderPrimitive.Control
				data-slot="slider-control"
				className="site-slider-control"
			>
				<SliderPrimitive.Track
					data-slot="slider-track"
					className="site-slider-track"
				>
					<SliderPrimitive.Indicator
						data-slot="slider-range"
						className="site-slider-range"
					/>
					<SliderPrimitive.Thumb
						data-slot="slider-thumb"
						className="site-slider-thumb"
						aria-label={thumbLabel}
					/>
				</SliderPrimitive.Track>
			</SliderPrimitive.Control>
		</SliderPrimitive.Root>
	);
}
