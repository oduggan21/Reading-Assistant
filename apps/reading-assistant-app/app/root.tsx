import { useMemo } from "react";
import {
  isRouteErrorResponse,
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
} from "react-router";
import type { Route } from "./+types/root";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SessionProvider } from "~/providers/session-provider";
import { AuthProvider } from "~/providers/auth-provider"; 
import "./globals.css";

// ✅ Fix: Return array directly, no parentheses wrapping
export const meta: Route.MetaFunction = () => {
  return [
    { charSet: "utf-8" },
    { title: "Interactive Audio Learner" },
    { name: "viewport", content: "width=device-width,initial-scale=1" },
  ];
};

export const links: Route.LinksFunction = () => [
  { rel: "preconnect", href: "https://fonts.googleapis.com" },
  { rel: "preconnect", href: "https://fonts.gstatic.com", crossOrigin: "anonymous" },
  {
    rel: "stylesheet",
    href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap",
  },
];

// ✅ Singleton pattern for QueryClient
let browserQueryClient: QueryClient | undefined = undefined;

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 60 * 1000,
      },
    },
  });
}

function getQueryClient() {
  if (typeof window === 'undefined') {
    return makeQueryClient();
  } else {
    if (!browserQueryClient) browserQueryClient = makeQueryClient();
    return browserQueryClient;
  }
}

export function Layout({ children }: { children: React.ReactNode }) {
  const queryClient = useMemo(() => getQueryClient(), []);
  
  return (
    <html lang="en" className="dark">
      <head>
        <Meta />
        <Links />
      </head>
      <body>
        <QueryClientProvider client={queryClient}>
          <AuthProvider>  {/* ✅ Add this */}
            <SessionProvider>{children}</SessionProvider>
          </AuthProvider>  {/* ✅ Add this */}
        </QueryClientProvider>
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function App() {
  return <Outlet />;
}
export function ErrorBoundary({ error }: Route.ErrorBoundaryProps) {
  let message = "Oops!";
  let details = "An unexpected error occurred.";
  let stack: string | undefined;

  if (isRouteErrorResponse(error)) {
    message = error.status === 404 ? "Page Not Found" : "Error";
    details = error.status === 404 ? "The requested page could not be found." : (error.statusText || details);
  } else if (import.meta.env.DEV && error instanceof Error) {
    details = error.message;
    stack = error.stack;
  }

  // ❌ Remove the <html> wrapper - it's already provided by Layout
  return (
    <main className="flex h-screen flex-col items-center justify-center text-center">
      <h1 className="text-4xl font-bold">{message}</h1>
      <p className="mt-2 text-lg text-muted-foreground">{details}</p>
      {stack && (
        <pre className="mt-4 w-full max-w-2xl overflow-x-auto rounded-md bg-muted p-4 text-left text-sm">
          <code>{stack}</code>
        </pre>
      )}
    </main>
  );
}