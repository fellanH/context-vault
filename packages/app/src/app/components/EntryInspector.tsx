import { useState } from "react";
import type { Entry } from "../lib/types";
import { useDeleteEntry } from "../lib/hooks";
import { ApiError } from "../lib/api";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "./ui/sheet";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { Separator } from "./ui/separator";
import { ScrollArea } from "./ui/scroll-area";
import {
  Edit,
  Trash2,
  Copy,
  FileText,
  Tag,
  Calendar,
  Hash,
  Link2,
  Loader2,
} from "lucide-react";
import { toast } from "sonner";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "./ui/alert-dialog";
import { Input } from "./ui/input";
import { Label } from "./ui/label";

interface EntryInspectorProps {
  entry: Entry | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function EntryInspector({ entry, open, onOpenChange }: EntryInspectorProps) {
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deleteConfirmText, setDeleteConfirmText] = useState("");
  const deleteEntry = useDeleteEntry();

  if (!entry) return null;

  const requiresConfirmation = entry.category === "knowledge" || entry.category === "entity";
  const confirmSlug = entry.source?.split("/").pop()?.replace(".md", "") || entry.id.slice(0, 8);

  const handleCopyJson = () => {
    navigator.clipboard.writeText(JSON.stringify(entry, null, 2));
    toast.success("JSON copied to clipboard");
  };

  const handleDelete = () => {
    if (requiresConfirmation && deleteConfirmText !== confirmSlug) {
      toast.error(`Type "${confirmSlug}" to confirm deletion`);
      return;
    }

    deleteEntry.mutate(entry.id, {
      onSuccess: () => {
        toast.success("Entry deleted");
        setDeleteDialogOpen(false);
        setDeleteConfirmText("");
        onOpenChange(false);
      },
      onError: (err) => {
        if (err instanceof ApiError) {
          toast.error("Failed to delete entry", { description: err.message });
        } else {
          toast.error("Failed to delete entry");
        }
      },
    });
  };

  const breadcrumbs = entry.source?.split("/") || [];

  return (
    <>
      <Sheet open={open} onOpenChange={onOpenChange}>
        <SheetContent className="sm:max-w-2xl">
          <SheetHeader>
            <div className="space-y-3">
              {/* Breadcrumbs */}
              <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                {breadcrumbs.map((crumb, i) => (
                  <div key={i} className="flex items-center gap-1.5">
                    <span className={i === breadcrumbs.length - 1 ? "text-foreground font-medium" : ""}>
                      {crumb}
                    </span>
                    {i < breadcrumbs.length - 1 && <span>/</span>}
                  </div>
                ))}
              </div>

              {/* Title */}
              <SheetTitle className="text-xl">{entry.title}</SheetTitle>

              {/* Actions */}
              <div className="flex items-center gap-2">
                <Button variant="outline" size="sm">
                  <Edit className="size-4 mr-2" />
                  Edit
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setDeleteDialogOpen(true)}
                >
                  <Trash2 className="size-4 mr-2" />
                  Delete
                </Button>
                <Button variant="outline" size="sm" onClick={handleCopyJson}>
                  <Copy className="size-4 mr-2" />
                  Copy JSON
                </Button>
              </div>
            </div>
          </SheetHeader>

          <Separator className="my-6" />

          <Tabs defaultValue="content" className="h-[calc(100vh-16rem)]">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="content">Content</TabsTrigger>
              <TabsTrigger value="metadata">Metadata</TabsTrigger>
            </TabsList>

            <TabsContent value="content" className="h-full">
              <ScrollArea className="h-full pr-4">
                <div className="prose prose-sm dark:prose-invert max-w-none">
                  <div
                    className="whitespace-pre-wrap font-['Inter']"
                    dangerouslySetInnerHTML={{
                      __html: entry.body
                        .replace(/^# (.+)$/gm, "<h1>$1</h1>")
                        .replace(/^## (.+)$/gm, "<h2>$1</h2>")
                        .replace(/^### (.+)$/gm, "<h3>$1</h3>")
                        .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
                        .replace(/`(.+?)`/g, "<code>$1</code>")
                        .replace(/^- (.+)$/gm, "<li>$1</li>")
                        .replace(/(<li>.*<\/li>)/s, "<ul>$1</ul>"),
                    }}
                  />
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent value="metadata" className="h-full">
              <ScrollArea className="h-full pr-4">
                <div className="space-y-4">
                  <div className="grid gap-3">
                    <div className="space-y-1.5">
                      <Label className="flex items-center gap-2 text-xs text-muted-foreground">
                        <Hash className="size-3" />
                        ID
                      </Label>
                      <code className="block px-3 py-2 bg-muted rounded text-xs font-mono">
                        {entry.id}
                      </code>
                    </div>

                    <div className="space-y-1.5">
                      <Label className="flex items-center gap-2 text-xs text-muted-foreground">
                        <FileText className="size-3" />
                        Category
                      </Label>
                      <div className="px-3 py-2">
                        <Badge variant="outline">{entry.category}</Badge>
                      </div>
                    </div>

                    <div className="space-y-1.5">
                      <Label className="flex items-center gap-2 text-xs text-muted-foreground">
                        <Tag className="size-3" />
                        Kind
                      </Label>
                      <div className="px-3 py-2">
                        <Badge
                          variant={
                            entry.category === "knowledge"
                              ? "default"
                              : entry.category === "entity"
                              ? "outline"
                              : "secondary"
                          }
                        >
                          {entry.kind}
                        </Badge>
                      </div>
                    </div>

                    <div className="space-y-1.5">
                      <Label className="flex items-center gap-2 text-xs text-muted-foreground">
                        <Tag className="size-3" />
                        Tags
                      </Label>
                      <div className="px-3 py-2 flex flex-wrap gap-1">
                        {entry.tags.map((tag) => (
                          <Badge key={tag} variant="outline" className="text-xs">
                            {tag}
                          </Badge>
                        ))}
                      </div>
                    </div>

                    <div className="space-y-1.5">
                      <Label className="flex items-center gap-2 text-xs text-muted-foreground">
                        <Calendar className="size-3" />
                        Created
                      </Label>
                      <div className="px-3 py-2 text-sm">
                        {entry.created.toLocaleString()}
                      </div>
                    </div>

                    <div className="space-y-1.5">
                      <Label className="flex items-center gap-2 text-xs text-muted-foreground">
                        <Calendar className="size-3" />
                        Updated
                      </Label>
                      <div className="px-3 py-2 text-sm">
                        {entry.updated.toLocaleString()}
                      </div>
                    </div>

                    {entry.source && (
                      <div className="space-y-1.5">
                        <Label className="flex items-center gap-2 text-xs text-muted-foreground">
                          <Link2 className="size-3" />
                          Source
                        </Label>
                        <code className="block px-3 py-2 bg-muted rounded text-xs font-mono">
                          {entry.source}
                        </code>
                      </div>
                    )}

                    {entry.metadata && Object.keys(entry.metadata).length > 0 && (
                      <div className="space-y-1.5">
                        <Label className="text-xs text-muted-foreground">Metadata</Label>
                        <pre className="px-3 py-2 bg-muted rounded text-xs font-mono overflow-x-auto">
                          {JSON.stringify(entry.metadata, null, 2)}
                        </pre>
                      </div>
                    )}
                  </div>
                </div>
              </ScrollArea>
            </TabsContent>
          </Tabs>
        </SheetContent>
      </Sheet>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Entry?</AlertDialogTitle>
            <AlertDialogDescription>
              {requiresConfirmation ? (
                <div className="space-y-3">
                  <p>
                    This {entry.category} entry is permanent. Type{" "}
                    <code className="px-1.5 py-0.5 bg-muted rounded text-xs font-mono">
                      {confirmSlug}
                    </code>{" "}
                    to confirm deletion.
                  </p>
                  <Input
                    value={deleteConfirmText}
                    onChange={(e) => setDeleteConfirmText(e.target.value)}
                    placeholder={confirmSlug}
                    className="font-mono"
                  />
                </div>
              ) : (
                <p>
                  This event entry will be permanently deleted. This action cannot be
                  undone.
                </p>
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setDeleteConfirmText("")}>
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction onClick={handleDelete} disabled={deleteEntry.isPending}>
              {deleteEntry.isPending && <Loader2 className="size-4 mr-2 animate-spin" />}
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
