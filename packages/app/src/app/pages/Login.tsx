import { useState } from "react";
import { Link, useNavigate } from "react-router";
import { useAuth } from "../lib/auth";
import { ApiError } from "../lib/api";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Key, Loader2, FolderOpen } from "lucide-react";
import { toast } from "sonner";

export function Login() {
  const { loginWithApiKey, loginWithLocalVault } = useAuth();
  const navigate = useNavigate();
  const [apiKey, setApiKey] = useState("");
  const [vaultDir, setVaultDir] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [localSubmitting, setLocalSubmitting] = useState(false);

  const handleApiKeySubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim() || isSubmitting) return;
    setIsSubmitting(true);
    try {
      await loginWithApiKey(apiKey.trim());
      toast.success("Authenticated successfully");
      navigate("/");
    } catch (err) {
      if (err instanceof ApiError) {
        if (err.status === 401) {
          toast.error("Invalid API key");
        } else {
          toast.error(err.message);
        }
      } else {
        toast.error("Failed to authenticate");
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleLocalSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (localSubmitting) return;
    setLocalSubmitting(true);
    try {
      await loginWithLocalVault(vaultDir.trim());
      toast.success("Connected to local vault");
      navigate("/");
    } catch (err) {
      if (err instanceof ApiError) {
        if (err.status === 404) {
          toast.error("Local vault requires context-mcp ui. Run: context-mcp ui");
        } else {
          toast.error(err.message);
        }
      } else {
        toast.error("Failed to connect to local vault");
      }
    } finally {
      setLocalSubmitting(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <div className="w-full max-w-sm space-y-6">
        <div className="text-center space-y-2">
          <h1 className="text-2xl font-bold">Context Vault</h1>
          <p className="text-sm text-muted-foreground">Sign in to your account</p>
        </div>

        <Card>
          <CardHeader className="pb-4">
            <CardTitle className="text-lg flex items-center gap-2">
              <FolderOpen className="size-5" />
              Local vault
            </CardTitle>
            <p className="text-sm text-muted-foreground">
              Connect to a local vault folder. No authentication required.
            </p>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleLocalSubmit} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="vaultDir">Vault folder path</Label>
                <Input
                  id="vaultDir"
                  type="text"
                  placeholder="e.g. ~/vault or /Users/me/vault"
                  value={vaultDir}
                  onChange={(e) => setVaultDir(e.target.value)}
                  disabled={localSubmitting}
                />
                <p className="text-xs text-muted-foreground">
                  Leave empty to use the default vault (~/vault)
                </p>
              </div>
              <Button type="submit" variant="default" className="w-full" disabled={localSubmitting}>
                {localSubmitting ? (
                  <>
                    <Loader2 className="size-4 mr-2 animate-spin" />
                    Connecting...
                  </>
                ) : (
                  "Connect to local vault"
                )}
              </Button>
            </form>
          </CardContent>
        </Card>

        <div className="relative">
          <div className="absolute inset-0 flex items-center">
            <span className="w-full border-t" />
          </div>
          <div className="relative flex justify-center text-xs uppercase">
            <span className="bg-background px-2 text-muted-foreground">Or</span>
          </div>
        </div>

        <Card>
          <CardHeader className="pb-4">
            <CardTitle className="text-lg flex items-center gap-2">
              <Key className="size-5" />
              API Key (hosted)
            </CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleApiKeySubmit} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="apiKey">API Key</Label>
                <div className="relative">
                  <Key className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
                  <Input
                    id="apiKey"
                    type="password"
                    placeholder="cv_..."
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    className="pl-9"
                    disabled={isSubmitting}
                  />
                </div>
              </div>
              <Button type="submit" variant="secondary" className="w-full" disabled={isSubmitting}>
                {isSubmitting ? (
                  <>
                    <Loader2 className="size-4 mr-2 animate-spin" />
                    Signing in...
                  </>
                ) : (
                  "Sign in with API key"
                )}
              </Button>
            </form>
          </CardContent>
        </Card>

        <p className="text-center text-sm text-muted-foreground">
          Don't have an account?{" "}
          <Link to="/register" className="text-foreground hover:underline font-medium">
            Register
          </Link>
        </p>
      </div>
    </div>
  );
}
