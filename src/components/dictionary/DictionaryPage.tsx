import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Plus,
  Search,
  Pencil,
  Trash2,
  X,
  Info,
  ArrowUpDown,
  BookOpen,
} from "lucide-react";
import { useSettings } from "@/hooks/useSettings";
import { Card, CardContent } from "@/components/shared/ui/card";
import { Button } from "@/components/shared/ui/button";
import { Input } from "@/components/shared/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/shared/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/shared/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { DictionaryEntryModal } from "./DictionaryEntryModal";
import { CustomWordEntry } from "@/bindings";
import { commands } from "@/bindings";
import { toast } from "sonner";

type SortOption = "newest" | "oldest" | "az" | "za";

function normalizeEntryKey(entry: CustomWordEntry): string {
  const aliases = (entry.aliases ?? [])
    .map((alias) => alias.trim().toLowerCase())
    .sort()
    .join("|");
  return `${entry.input.trim().toLowerCase()}::${entry.replacement.trim().toLowerCase()}::${entry.is_replacement}::${aliases}`;
}

function entriesEqual(a: CustomWordEntry, b: CustomWordEntry): boolean {
  return normalizeEntryKey(a) === normalizeEntryKey(b);
}

export function DictionaryPage() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettings();
  const [searchQuery, setSearchQuery] = useState("");
  const [sortOption, setSortOption] = useState<SortOption>("newest");
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isHelpModalOpen, setIsHelpModalOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<CustomWordEntry | undefined>(
    undefined,
  );
  const [entryToDelete, setEntryToDelete] = useState<CustomWordEntry | null>(
    null,
  );
  const [isHeroDismissed, setIsHeroDismissed] = useState(() => {
    return localStorage.getItem("dictionary-hero-dismissed") === "true";
  });

  const entries = useMemo(
    () => settings?.dictionary || [],
    [settings?.dictionary],
  );

  // Filter entries based on search
  const filteredEntries = useMemo(() => {
    let result = entries;
    if (searchQuery) {
      const lowerQuery = searchQuery.toLowerCase();
      result = result.filter(
        (entry) =>
          entry.input.toLowerCase().includes(lowerQuery) ||
          entry.replacement.toLowerCase().includes(lowerQuery) ||
          (entry.aliases ?? []).some((alias) =>
            alias.toLowerCase().includes(lowerQuery),
          ),
      );
    }
    return result;
  }, [entries, searchQuery]);

  // Sort filtered entries
  const sortedEntries = useMemo(() => {
    const sorted = [...filteredEntries];
    switch (sortOption) {
      case "az":
        sorted.sort((a, b) => a.input.localeCompare(b.input));
        break;
      case "za":
        sorted.sort((a, b) => b.input.localeCompare(a.input));
        break;
      case "oldest":
        // Keep original order (oldest first since entries are appended)
        break;
      case "newest":
      default:
        // Reverse to show newest first
        sorted.reverse();
        break;
    }
    return sorted;
  }, [filteredEntries, sortOption]);

  const handleSaveEntry = async (entry: CustomWordEntry) => {
    let newEntries = [...entries];
    if (editingEntry) {
      const index = newEntries.findIndex((e) => entriesEqual(e, editingEntry));
      if (index !== -1) {
        newEntries[index] = entry;
      }
    } else {
      newEntries.push(entry);
    }

    try {
      await commands.updateCustomWords(newEntries);
      updateSetting("dictionary", newEntries);
      toast.success(t("dictionary.saved_success", "Dictionary updated"));
    } catch (error) {
      console.error("Failed to update dictionary:", error);
      toast.error(t("dictionary.save_failed", "Failed to update dictionary"));
    }
  };

  const handleDeleteEntry = async () => {
    if (!entryToDelete) return;

    const newEntries = entries.filter((e) => !entriesEqual(e, entryToDelete));

    try {
      await commands.updateCustomWords(newEntries);
      updateSetting("dictionary", newEntries);
      toast.success(t("dictionary.deleted_success", "Entry deleted"));
    } catch (error) {
      console.error("Failed to delete entry:", error);
      toast.error(t("dictionary.delete_failed", "Failed to delete entry"));
    } finally {
      setEntryToDelete(null);
    }
  };

  const handleRemoveAlias = async (
    entry: CustomWordEntry,
    aliasToRemove: string,
  ) => {
    const nextAliases = (entry.aliases ?? []).filter(
      (alias) => alias !== aliasToRemove,
    );
    const newEntries = entries.map((candidate) =>
      entriesEqual(candidate, entry)
        ? {
            ...candidate,
            aliases: nextAliases,
          }
        : candidate,
    );

    try {
      await commands.updateCustomWords(newEntries);
      updateSetting("dictionary", newEntries);
      toast.success(t("dictionary.alias_removed", "Alias removed"));
    } catch (error) {
      console.error("Failed to remove alias:", error);
      toast.error(
        t("dictionary.alias_remove_failed", "Failed to remove alias"),
      );
    }
  };

  const openEditModal = (entry: CustomWordEntry) => {
    setEditingEntry(entry);
    setIsModalOpen(true);
  };

  const openAddModal = () => {
    setEditingEntry(undefined);
    setIsModalOpen(true);
  };

  const dismissHero = () => {
    setIsHeroDismissed(true);
    localStorage.setItem("dictionary-hero-dismissed", "true");
  };

  const highlightText = (text: string, query: string) => {
    if (!query || query.length === 0) return text;

    const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const parts = text.split(new RegExp(`(${escapedQuery})`, "gi"));

    return parts.map((part, i) =>
      part.toLowerCase() === query.toLowerCase() ? (
        <span
          key={i}
          className="bg-yellow-500/30 text-foreground rounded-[2px] px-0.5 -mx-0.5 font-medium"
        >
          {part}
        </span>
      ) : (
        part
      ),
    );
  };

  // Static example pills for hero banner - showcase value across different use cases
  const examplePills = [
    "chat gpt → ChatGPT", // Multi-word phrase matching (common user need)
    "co dictate → Codictate", // Formatting/branding
    "my email → john@example.com", // Personal info shortcuts
    "btw → by the way", // Abbreviation expansion
    "Anthropic", // Tech/company names that might be misheard
    "EBITDA", // Industry jargon
  ];

  const quickTips = [
    {
      title: t("dictionary.quick_tips.tip1_title", "Use canonical + aliases"),
      description: t(
        "dictionary.quick_tips.tip1_description",
        "Example: input 'shadcn' with aliases 'shad cn' and 'shad c n'.",
      ),
    },
    {
      title: t(
        "dictionary.quick_tips.tip2_title",
        "Use replacement mode for shortcuts",
      ),
      description: t(
        "dictionary.quick_tips.tip2_description",
        "Turn on \"Replace with different text\" for entries like 'btw' -> 'by the way'.",
      ),
    },
    {
      title: t(
        "dictionary.quick_tips.tip3_title",
        "Matching is case-insensitive",
      ),
      description: t(
        "dictionary.quick_tips.tip3_description",
        "Set the replacement text with your desired capitalization (for example, ChatGPT).",
      ),
    },
    {
      title: t(
        "dictionary.quick_tips.tip4_title",
        "Refine from real dictation",
      ),
      description: t(
        "dictionary.quick_tips.tip4_description",
        "If a term misses, add 1-2 aliases based on what you actually see in transcripts.",
      ),
    },
  ];

  const quickExamples = [
    t("dictionary.help_modal.example1", "shad cn -> shadcn"),
    t("dictionary.help_modal.example2", "chat gpt -> ChatGPT"),
    t("dictionary.help_modal.example3", "btw -> by the way"),
    t("dictionary.help_modal.example4", "my email -> john@example.com"),
  ];

  return (
    <TooltipProvider delayDuration={300}>
      <div className="flex flex-col h-full overflow-hidden w-full relative">
        {/* Static content: Header, Hero, Toolbar */}
        <div className="flex-none max-w-3xl w-full mx-auto pt-4 px-4 flex flex-col gap-6">
          {/* Header */}
          <div className="flex justify-between items-center">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">
                {t("dictionary.title", "Dictionary")}
              </h1>
            </div>
            <div className="flex items-center gap-2">
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="outline"
                    size="icon"
                    className="h-8 w-8"
                    onClick={() => setIsHelpModalOpen(true)}
                  >
                    <Info className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{t("dictionary.help_tooltip", "How to use Dictionary")}</p>
                </TooltipContent>
              </Tooltip>
              <Button onClick={openAddModal} size="sm" className="shadow-sm">
                <Plus className="mr-1.5 h-4 w-4" />
                {t("dictionary.add_new", "Add new")}
              </Button>
            </div>
          </div>

          {/* Hero Banner */}
          {!isHeroDismissed && (
            <Card className="relative bg-gradient-to-br from-muted/50 to-muted/30 border-border/60 shadow-sm overflow-hidden">
              <button
                onClick={dismissHero}
                className="absolute top-3 right-3 p-1 rounded-md text-muted-foreground/60 hover:text-muted-foreground hover:bg-muted/50 transition-colors"
              >
                <X className="h-4 w-4" />
              </button>
              <CardContent className="p-6 pt-5">
                <h2 className="text-xl font-medium italic text-foreground/90 mb-2">
                  {t(
                    "dictionary.hero_tagline",
                    "Codictate learns how you speak.",
                  )}
                </h2>
                <p className="text-sm text-muted-foreground leading-relaxed mb-4 max-w-lg">
                  {t(
                    "dictionary.hero_description",
                    "Teach Codictate your unique words — names, jargon, or terms it might mishear.",
                  )}
                </p>
                <div className="flex flex-wrap gap-2 mb-4">
                  {examplePills.map((pill, i) => (
                    <span
                      key={i}
                      className="px-3 py-1.5 text-sm bg-background/80 border border-border/60 rounded-full text-foreground/80"
                    >
                      {pill}
                    </span>
                  ))}
                </div>
                <Button
                  onClick={openAddModal}
                  variant="secondary"
                  size="sm"
                  className="bg-foreground text-background hover:bg-foreground/90 shadow-sm"
                >
                  {t("dictionary.add_new_word", "Add new word")}
                </Button>
              </CardContent>
            </Card>
          )}

          {/* Toolbar: Search + Sort */}
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              {/* Always-visible Search */}
              <div className="relative">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground/70" />
                <Input
                  placeholder={t("dictionary.search_placeholder", "Search...")}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-8 pr-8 h-9 w-88 bg-background/50"
                />
                {searchQuery && (
                  <button
                    onClick={() => setSearchQuery("")}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground/60 hover:text-muted-foreground"
                  >
                    <X className="h-3.5 w-3.5" />
                  </button>
                )}
              </div>

              {/* Results count when searching */}
              {searchQuery && (
                <span className="text-xs text-muted-foreground">
                  {t("dictionary.found_results", {
                    count: filteredEntries.length,
                  })}
                </span>
              )}
            </div>

            {/* Sort Dropdown */}
            <Select
              value={sortOption}
              onValueChange={(val) => setSortOption(val as SortOption)}
            >
              <SelectTrigger className="w-[160px] h-9 text-sm bg-transparent border-none shadow-none hover:bg-muted/50">
                <ArrowUpDown className="h-4 w-4 mr-2 text-muted-foreground" />
                <SelectValue />
              </SelectTrigger>
              <SelectContent align="end" className="min-w-[160px]">
                <SelectItem value="newest">
                  {t("dictionary.sort.newest", "Newest first")}
                </SelectItem>
                <SelectItem value="oldest">
                  {t("dictionary.sort.oldest", "Oldest first")}
                </SelectItem>
                <SelectItem value="az">
                  {t("dictionary.sort.az", "A-Z")}
                </SelectItem>
                <SelectItem value="za">
                  {t("dictionary.sort.za", "Z-A")}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* Scrollable Entries List - fills remaining space */}
        <div className="flex-1 overflow-y-auto pb-12 mt-6 scrollbar-thin scrollbar-thumb-muted/50 scrollbar-track-transparent">
          <div className="max-w-3xl w-full mx-auto px-4 min-h-full flex flex-col">
            <Card className="w-full bg-card border shadow-sm rounded-xl overflow-hidden flex-1 flex flex-col">
              <CardContent className="p-0 flex-1 flex flex-col">
                {sortedEntries.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-16 px-4 text-center">
                    <div className="bg-muted/50 p-4 rounded-full mb-4">
                      <BookOpen className="w-8 h-8 text-muted-foreground/50" />
                    </div>
                    <p className="text-muted-foreground font-medium mb-1">
                      {searchQuery
                        ? t("dictionary.no_match", "No matching entries found.")
                        : t("dictionary.empty", "No dictionary entries yet.")}
                    </p>
                    {!searchQuery && (
                      <p className="text-xs text-muted-foreground/70 max-w-xs">
                        {t(
                          "dictionary.empty_description",
                          "Add words to help Codictate learn your vocabulary.",
                        )}
                      </p>
                    )}
                  </div>
                ) : (
                  <div className="divide-y divide-border/40">
                    {sortedEntries.map((entry, index) => (
                      <div
                        key={`${entry.input}-${index}`}
                        className="group flex items-start justify-between px-5 py-4 hover:bg-accent/30 transition-colors gap-4"
                      >
                        {/* Entry text - equal columns that wrap */}
                        <div className="flex items-start gap-3 min-w-0 flex-1 text-sm">
                          {/* Input column - takes 50% */}
                          <div className="flex-1 min-w-0 basis-1/2">
                            <span className="font-medium break-words">
                              {highlightText(entry.input, searchQuery)}
                            </span>
                            {(entry.aliases ?? []).length > 0 && (
                              <div className="mt-1 flex flex-wrap gap-1.5">
                                {(entry.aliases ?? []).map(
                                  (alias, aliasIndex) => (
                                    <span
                                      key={`${alias}-${aliasIndex}`}
                                      className="inline-flex items-center gap-1 rounded-full border border-border/70 bg-muted/50 px-2 py-0.5 text-[11px] text-muted-foreground"
                                    >
                                      <span>
                                        {highlightText(alias, searchQuery)}
                                      </span>
                                      <button
                                        type="button"
                                        onClick={() =>
                                          handleRemoveAlias(entry, alias)
                                        }
                                        className="inline-flex h-3.5 w-3.5 items-center justify-center rounded-full text-muted-foreground/50 transition-all hover:bg-muted hover:text-foreground/85 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:text-foreground/85 opacity-45 hover:opacity-100"
                                        title={t(
                                          "dictionary.remove_alias",
                                          "Remove alias",
                                        )}
                                        aria-label={t(
                                          "dictionary.remove_alias",
                                          "Remove alias",
                                        )}
                                      >
                                        <X className="h-2.5 w-2.5" />
                                      </button>
                                    </span>
                                  ),
                                )}
                              </div>
                            )}
                          </div>
                          {entry.replacement &&
                            entry.replacement !== entry.input && (
                              <>
                                <span className="text-muted-foreground shrink-0 pt-0.5">
                                  →
                                </span>
                                {/* Replacement column - takes 50% */}
                                <div className="flex-1 min-w-0 basis-1/2">
                                  <span className="font-medium break-words">
                                    {highlightText(
                                      entry.replacement,
                                      searchQuery,
                                    )}
                                  </span>
                                </div>
                              </>
                            )}
                        </div>

                        {/* Right side: Icons with margin from text */}
                        <div className="flex items-center gap-1 shrink-0 ml-4 text-muted-foreground/50 hover:text-muted-foreground transition-colors">
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-8 w-8"
                                onClick={() => openEditModal(entry)}
                              >
                                <Pencil className="h-4 w-4" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>
                              <p>{t("dictionary.edit_entry", "Edit Entry")}</p>
                            </TooltipContent>
                          </Tooltip>

                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-8 w-8 hover:text-destructive hover:bg-destructive/10"
                                onClick={() => setEntryToDelete(entry)}
                              >
                                <Trash2 className="h-4 w-4" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>
                              <p>{t("common.delete", "Delete")}</p>
                            </TooltipContent>
                          </Tooltip>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          </div>
        </div>

        {/* Bottom fade/blur effect - at page level like Home.tsx */}
        <div className="absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-background via-background/60 to-transparent pointer-events-none z-10 backdrop-blur-[1px]" />

        <DictionaryEntryModal
          isOpen={isModalOpen}
          onClose={() => setIsModalOpen(false)}
          onSave={handleSaveEntry}
          initialEntry={editingEntry}
          existingEntries={entries}
        />

        <Dialog open={isHelpModalOpen} onOpenChange={setIsHelpModalOpen}>
          <DialogContent className="sm:max-w-[620px] border-border/60 shadow-2xl dark:border-border dark:shadow-black/50 dark:bg-card">
            <DialogHeader>
              <DialogTitle>
                {t(
                  "dictionary.help_modal.title",
                  "Get Better Dictation Results",
                )}
              </DialogTitle>
              <DialogDescription>
                {t(
                  "dictionary.help_modal.description",
                  "Use these quick rules to make Dictionary entries trigger more reliably.",
                )}
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4 py-2">
              <div className="grid gap-2 sm:grid-cols-2">
                {quickTips.map((tip, index) => (
                  <div
                    key={tip.title}
                    className="rounded-lg border border-border/60 bg-muted/20 px-3 py-2.5"
                  >
                    <p className="text-xs font-semibold text-foreground/90">
                      {index + 1}. {tip.title}
                    </p>
                    <p className="text-xs text-muted-foreground leading-relaxed mt-1">
                      {tip.description}
                    </p>
                  </div>
                ))}
              </div>

              <div className="rounded-lg border border-border/60 bg-background/80 p-3">
                <p className="text-[11px] uppercase tracking-wide text-muted-foreground/70 font-semibold mb-2">
                  {t("dictionary.help_modal.examples_label", "Quick examples")}
                </p>
                <div className="flex flex-wrap gap-2">
                  {quickExamples.map((example) => (
                    <span
                      key={example}
                      className="inline-flex items-center rounded-full border border-border/70 bg-muted/40 px-2.5 py-1 text-xs text-foreground/85"
                    >
                      {example}
                    </span>
                  ))}
                </div>
              </div>
            </div>

            <DialogFooter className="gap-2 sm:gap-2">
              <Button
                variant="outline"
                onClick={() => setIsHelpModalOpen(false)}
              >
                {t("common.cancel", "Cancel")}
              </Button>
              <Button
                onClick={() => {
                  setIsHelpModalOpen(false);
                  openAddModal();
                }}
                className="bg-foreground text-background hover:bg-foreground/90"
              >
                {t("dictionary.help_modal.cta", "Add an entry now")}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <AlertDialog
          open={!!entryToDelete}
          onOpenChange={() => setEntryToDelete(null)}
        >
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>
                {t("common.are_you_sure", "Are you sure?")}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {t(
                  "dictionary.delete_confirm",
                  "This action cannot be undone. This will permanently delete this dictionary entry.",
                )}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>
                {t("common.cancel", "Cancel")}
              </AlertDialogCancel>
              <AlertDialogAction
                onClick={handleDeleteEntry}
                className="bg-destructive hover:bg-destructive/90"
              >
                {t("common.delete", "Delete")}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </TooltipProvider>
  );
}
