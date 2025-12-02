import { createContext, useContext, useState, ReactNode } from "react";
import { 
  useSignupHandler, 
  useLoginHandler, 
  useLogoutHandler 
} from "@reading-assistant/query/auth";
import type { AuthResponse } from "@reading-assistant/query/schemas";

type AuthContextType = {
  user: AuthResponse | null;
  login: (email: string, password: string) => Promise<void>;
  signup: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  isLoading: boolean;
};

const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthResponse | null>(null);
  
  const signupMutation = useSignupHandler();
  const loginMutation = useLoginHandler();
  const logoutMutation = useLogoutHandler();
  
  const isLoading = signupMutation.isPending || loginMutation.isPending || logoutMutation.isPending;
  
  const signup = async (email: string, password: string) => {
    const response = await signupMutation.mutateAsync({ data: { email, password } });
    setUser(response);
  };
  
  const login = async (email: string, password: string) => {
    const response = await loginMutation.mutateAsync({ data: { email, password } });
    setUser(response);
  };
  
  const logout = async () => {
    await logoutMutation.mutateAsync();
    setUser(null);
  };
  
  return (
    <AuthContext.Provider value={{ user, login, signup, logout, isLoading }}>
      {children}
    </AuthContext.Provider>
  );
}

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};