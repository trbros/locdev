import { notFound } from "next/navigation";
/** Keep unknown paths inside the locale layout and its translated not-found UI. */
export default function UnknownPage() {
	notFound();
}
