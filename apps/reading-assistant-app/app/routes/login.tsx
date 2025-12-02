import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAuth } from "~/providers/auth-provider";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { BookHeadphones } from "lucide-react";

export default function LoginPage() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [isSignup, setIsSignup] = useState(false);
  const [error, setError] = useState("");
  
  const { login, signup, isLoading } = useAuth();
  const navigate = useNavigate();
  
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    
    try {
      if (isSignup) {
        await signup(email, password);
      } else {
        await login(email, password);
      }
      navigate("/");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Authentication failed");
    }
  };
  
  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4">
      <div className="w-full max-w-md">
        {/* Header */}
        <div className="mb-8 text-center">
          <div className="inline-flex items-center gap-2 rounded-full border bg-card px-3 py-1.5 text-sm text-muted-foreground mb-4">
            <BookHeadphones className="h-4 w-4 text-primary" />
            <span>Sage</span>
          </div>
          <h1 className="text-3xl font-bold">
            {isSignup ? "Create an account" : "Welcome back"}
          </h1>
          <p className="mt-2 text-muted-foreground">
            {isSignup 
              ? "Sign up to start your interactive learning journey" 
              : "Sign in to continue your learning"}
          </p>
        </div>

        {/* Auth Card */}
        <Card>
          <CardHeader>
            <CardTitle>{isSignup ? "Sign Up" : "Log In"}</CardTitle>
            <CardDescription>
              {isSignup 
                ? "Enter your details to create your account" 
                : "Enter your credentials to access your account"}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleSubmit} className="space-y-4">
              {error && (
                <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
                  {error}
                </div>
              )}
              
              <div className="space-y-2">
                <label htmlFor="email" className="text-sm font-medium">
                  Email
                </label>
                <Input
                  id="email"
                  type="email"
                  placeholder="you@example.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  required
                  disabled={isLoading}
                />
              </div>
              
              <div className="space-y-2">
                <label htmlFor="password" className="text-sm font-medium">
                  Password
                </label>
                <Input
                  id="password"
                  type="password"
                  placeholder="••••••••"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  required
                  disabled={isLoading}
                  minLength={6}
                />
                {isSignup && (
                  <p className="text-xs text-muted-foreground">
                    Must be at least 6 characters
                  </p>
                )}
              </div>
              
              <Button 
                type="submit" 
                className="w-full" 
                disabled={isLoading}
              >
                {isLoading ? "Please wait..." : (isSignup ? "Sign Up" : "Log In")}
              </Button>
              
              <Button
                type="button"
                variant="ghost"
                className="w-full"
                onClick={() => {
                  setIsSignup(!isSignup);
                  setError("");
                }}
                disabled={isLoading}
              >
                {isSignup 
                  ? "Already have an account? Log in" 
                  : "Don't have an account? Sign up"}
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </main>
  );
}