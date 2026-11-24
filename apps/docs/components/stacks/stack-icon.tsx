import {
	Angular,
	Astro,
	Bun,
	Deno,
	Django,
	DotNet,
	Elysiajs,
	Ember,
	Expressjs,
	FastAPI,
	Fastify,
	Flask,
	Gatsby,
	Go,
	Hono,
	HTML5,
	Hugo,
	Java,
	JavaScript,
	Laravel,
	Nestjs,
	Nextjs,
	Nodejs,
	Nuxt,
	PHP,
	Preact,
	Python,
	Qwik,
	Rails,
	React,
	Redwoodjs,
	Remix,
	Ruby,
	Rust,
	Solidjs,
	Spring,
	Svelte,
	TypeScript,
	Vite,
	Vue,
	WordPress,
} from "@/components/icons";

const stackIcons = {
	Angular,
	Astro,
	Bun,
	Deno,
	Django,
	DotNet,
	Elysiajs,
	Ember,
	Expressjs,
	FastAPI,
	Fastify,
	Flask,
	Gatsby,
	Go,
	HTML5,
	Hono,
	Hugo,
	Java,
	JavaScript,
	Laravel,
	Nestjs,
	Nextjs,
	Nodejs,
	Nuxt,
	PHP,
	Preact,
	Python,
	Qwik,
	Rails,
	React,
	Redwoodjs,
	Remix,
	Ruby,
	Rust,
	Solidjs,
	Spring,
	Svelte,
	TypeScript,
	Vite,
	Vue,
	WordPress,
};

import type { Stack } from "@/lib/stacks";

export function StackIcon({
	stack,
	className = "size-6",
}: {
	stack: Stack;
	className?: string;
}) {
	const Icon = stackIcons[stack.icon];
	return (
		<span className={`stack-icon ${className}`} aria-hidden="true">
			<Icon className="size-full" focusable="false" />
		</span>
	);
}
