import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Info, ArrowRight, X, Plus } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { Button } from "@/components/shared/ui/button";
import { Input } from "@/components/shared/ui/input";
import { Label } from "@/components/shared/ui/label";
import { Switch } from "@/components/shared/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { CustomWordEntry } from "@/bindings";
import { cn } from "@/lib/utils";
import { isDuplicateEntry, normalizeAliases } from "@/utils/dictionaryUtils";

// Character limits based on UX best practices:
// - Input (trigger phrase): 100 chars (~10-15 words) - longer phrases are unlikely to be consistently spoken
// - Replacement: 300 chars - more flexibility for expanded text, addresses, email signatures, etc.
const INPUT_MAX_LENGTH = 100;
const REPLACEMENT_MAX_LENGTH = 300;
const ALIAS_MAX_LENGTH = 100;
const MAX_ALIASES_COUNT = 8;

function parseAliasTokens(raw: string): string[] {
  return raw
    .split(/[\n,]/)
    .map((alias) => alias.trim())
    .filter((alias) => alias.length > 0);
}

interface DictionaryEntryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (entry: CustomWordEntry) => void;
  initialEntry?: CustomWordEntry;
  existingEntries?: CustomWordEntry[];
}

// Auto-growing textarea component
function AutoGrowTextarea({
  value,
  onChange,
  onKeyDown,
  placeholder,
  maxLength,
  autoFocus,
  id,
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  onKeyDown?: (e: React.KeyboardEvent) => void;
  placeholder?: string;
  maxLength: number;
  autoFocus?: boolean;
  id?: string;
  className?: string;
}) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const isNearLimit = value.length >= maxLength * 0.8;
  const isAtLimit = value.length >= maxLength;

  // Auto-resize textarea based on content
  const adjustHeight = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    // Reset height to calculate scrollHeight correctly
    textarea.style.height = "auto";

    // Calculate line height (approx 20px per line)
    const lineHeight = 20;
    const minRows = 1;
    const maxRows = 3;
    const minHeight = lineHeight * minRows + 16; // 16px for padding
    const maxHeight = lineHeight * maxRows + 16;

    // Set height based on content, clamped to min/max
    const newHeight = Math.min(
      Math.max(textarea.scrollHeight, minHeight),
      maxHeight,
    );
    textarea.style.height = `${newHeight}px`;
  }, []);

  useEffect(() => {
    adjustHeight();
  }, [value, adjustHeight]);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    // Enforce character limit
    if (newValue.length <= maxLength) {
      onChange(newValue);
    }
  };

  return (
    <div className="relative">
      <textarea
        ref={textareaRef}
        id={id}
        value={value}
        onChange={handleChange}
        onKeyDown={onKeyDown}
        placeholder={placeholder}
        autoFocus={autoFocus}
        spellCheck={false}
        autoCorrect="off"
        autoCapitalize="none"
        autoComplete="off"
        maxLength={maxLength}
        className={cn(
          "flex w-full rounded-xl border border-input bg-transparent px-3 py-2 text-sm shadow-xs transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 resize-none overflow-hidden",
          isAtLimit && "border-amber-500/50 focus-visible:ring-amber-500/50",
          isNearLimit && "pb-5", // Extra bottom padding when counter is visible
          className,
        )}
        rows={1}
        style={{ minHeight: "36px" }}
      />
      {/* Character counter - shows when nearing or at limit */}
      {isNearLimit && (
        <span
          className={cn(
            "absolute right-2 bottom-1.5 text-[10px] font-medium",
            isAtLimit ? "text-amber-500" : "text-muted-foreground/50",
          )}
        >
          {value.length}/{maxLength}
        </span>
      )}
    </div>
  );
}

export function DictionaryEntryModal({
  isOpen,
  onClose,
  onSave,
  initialEntry,
  existingEntries = [],
}: DictionaryEntryModalProps) {
  const { t } = useTranslation();
  const [input, setInput] = useState("");
  const [replacement, setReplacement] = useState("");
  const [aliases, setAliases] = useState<string[]>([]);
  const [aliasDraft, setAliasDraft] = useState("");
  const [isReplacement, setIsReplacement] = useState(false);
  const [duplicateError, setDuplicateError] = useState(false);

  const addAliasesFromRaw = useCallback(
    (raw: string) => {
      const tokens = parseAliasTokens(raw);
      if (tokens.length === 0) return;
      setAliases((prev) => {
        const merged = normalizeAliases([...prev, ...tokens], input);
        return merged.slice(0, MAX_ALIASES_COUNT);
      });
    },
    [input],
  );

  // Check for duplicates when input changes
  useEffect(() => {
    if (!input.trim()) {
      setDuplicateError(false);
      return;
    }
    setDuplicateError(
      isDuplicateEntry(input, aliases, existingEntries, initialEntry),
    );
  }, [input, aliases, existingEntries, initialEntry]);

  useEffect(() => {
    setAliases((prev) => normalizeAliases(prev, input));
  }, [input]);

  useEffect(() => {
    if (isOpen) {
      if (initialEntry) {
        setInput(initialEntry.input);
        setReplacement(initialEntry.replacement);
        setAliases(
          normalizeAliases(initialEntry.aliases ?? [], initialEntry.input),
        );
        setAliasDraft("");
        setIsReplacement(initialEntry.is_replacement);
      } else {
        setInput("");
        setReplacement("");
        setAliases([]);
        setAliasDraft("");
        setIsReplacement(false);
      }
    }
  }, [isOpen, initialEntry]);

  const canSave = useCallback(() => {
    if (!input.trim()) return false;
    if (isReplacement && !replacement.trim()) return false;
    if (duplicateError) return false;
    return true;
  }, [input, replacement, isReplacement, duplicateError]);

  const handleSave = useCallback(() => {
    if (!canSave()) return;

    onSave({
      input: input.trim(),
      aliases,
      // When replacement mode is on, use custom replacement; otherwise same as input
      replacement: isReplacement ? replacement.trim() : input.trim(),
      is_replacement: isReplacement,
    });
    onClose();
  }, [input, replacement, aliases, isReplacement, canSave, onSave, onClose]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      // Save on Enter (since we're using single-line style now) or Cmd/Ctrl + Enter
      if (e.key === "Enter" && !e.shiftKey && canSave()) {
        e.preventDefault();
        handleSave();
      }
    },
    [canSave, handleSave],
  );

  const handleAliasKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter" || e.key === ",") {
        e.preventDefault();
        addAliasesFromRaw(aliasDraft);
        setAliasDraft("");
        return;
      }

      if (
        e.key === "Backspace" &&
        aliasDraft.length === 0 &&
        aliases.length > 0
      ) {
        e.preventDefault();
        setAliases((prev) => prev.slice(0, -1));
      }
    },
    [addAliasesFromRaw, aliasDraft, aliases.length],
  );

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[520px] border-border/60 shadow-2xl dark:border-border dark:shadow-black/50 dark:bg-card">
        <DialogHeader className="pb-2">
          <DialogTitle className="text-lg">
            {initialEntry
              ? t("dictionary.edit_entry", "Edit Entry")
              : t("dictionary.add_entry", "Add Entry")}
          </DialogTitle>
          <DialogDescription className="text-sm">
            {t(
              "dictionary.modal_description",
              "Add a word or phrase you want Codictate to recognize.",
            )}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-5 py-4">
          {/* Replacement Toggle */}
          <TooltipProvider delayDuration={800}>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Label
                  htmlFor="is-replacement"
                  className="text-sm font-medium cursor-pointer"
                >
                  {t(
                    "dictionary.replace_with_text",
                    "Replace with different text",
                  )}
                </Label>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      tabIndex={-1}
                      className="text-muted-foreground/60 hover:text-muted-foreground transition-colors"
                    >
                      <Info className="h-4 w-4" />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent
                    side="top"
                    className="max-w-[320px] bg-foreground text-background p-3"
                  >
                    <p className="text-sm leading-relaxed">
                      {t(
                        "dictionary.replacement_description",
                        "Enable this to replace phrases with different text. Examples: 'chat gpt' → 'ChatGPT', 'btw' → 'by the way', 'my email' → 'john@example.com'. Tip: match the word count (e.g. 'chat gpt' is 2 words, so it matches when you say 2 words).",
                      )}
                    </p>
                  </TooltipContent>
                </Tooltip>
              </div>
              <Switch
                id="is-replacement"
                checked={isReplacement}
                onCheckedChange={(checked) => {
                  setIsReplacement(checked);
                  // Clear replacement when toggling off to avoid stale data
                  if (!checked) setReplacement("");
                }}
              />
            </div>
          </TooltipProvider>

          {/* Input Fields - conditional based on isReplacement */}
          {isReplacement ? (
            /* Two-field layout for replacement mode */
            <div className="space-y-4">
              {/* Side by side layout with arrow */}
              <div className="flex items-start gap-3">
                <div className="flex-1 space-y-1.5">
                  <Label
                    htmlFor="input"
                    className="text-xs font-medium text-muted-foreground"
                  >
                    {t("dictionary.when_i_say", "When I say...")}
                  </Label>
                  <AutoGrowTextarea
                    id="input"
                    autoFocus
                    value={input}
                    onChange={setInput}
                    onKeyDown={handleKeyDown}
                    className={
                      duplicateError
                        ? "border-destructive focus-visible:ring-destructive"
                        : undefined
                    }
                    placeholder={t("dictionary.input_example", "e.g. my email")}
                    maxLength={INPUT_MAX_LENGTH}
                  />
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0 mt-7" />
                <div className="flex-1 space-y-1.5">
                  <Label
                    htmlFor="replacement"
                    className="text-xs font-medium text-muted-foreground"
                  >
                    {t("dictionary.replace_with", "Replace with...")}
                  </Label>
                  <AutoGrowTextarea
                    id="replacement"
                    value={replacement}
                    onChange={setReplacement}
                    onKeyDown={handleKeyDown}
                    placeholder={t(
                      "dictionary.replacement_example",
                      "e.g. john@example.com",
                    )}
                    maxLength={REPLACEMENT_MAX_LENGTH}
                  />
                </div>
              </div>
              {duplicateError && (
                <p className="text-xs text-destructive">
                  {t(
                    "dictionary.duplicate_error",
                    "This term or alias already exists in your dictionary",
                  )}
                </p>
              )}
            </div>
          ) : (
            /* Single-field layout for vocabulary mode */
            <div className="space-y-2">
              <Label htmlFor="input" className="text-sm font-medium">
                {t("dictionary.word_to_recognize", "Word to recognize")}
              </Label>
              <AutoGrowTextarea
                id="input"
                autoFocus
                value={input}
                onChange={setInput}
                onKeyDown={handleKeyDown}
                placeholder={t(
                  "dictionary.input_placeholder",
                  "e.g. Codictate, EBITDA, John Smith",
                )}
                maxLength={INPUT_MAX_LENGTH}
                className={
                  duplicateError
                    ? "border-destructive focus-visible:ring-destructive"
                    : undefined
                }
              />
              {duplicateError && (
                <p className="text-xs text-destructive">
                  {t(
                    "dictionary.duplicate_error",
                    "This term or alias already exists in your dictionary",
                  )}
                </p>
              )}
            </div>
          )}

          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <Label htmlFor="aliases" className="text-sm font-medium">
                {isReplacement
                  ? t(
                      "dictionary.aliases_label_replacement",
                      "Also when I say... (optional)",
                    )
                  : t("dictionary.aliases_label", "Aliases (optional)")}
              </Label>
              <TooltipProvider delayDuration={400}>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      tabIndex={-1}
                      className="text-muted-foreground/60 hover:text-muted-foreground transition-colors"
                      aria-label={t(
                        "dictionary.aliases_help_tooltip_label",
                        "Alias tips",
                      )}
                    >
                      <Info className="h-4 w-4" />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent
                    side="top"
                    className="max-w-[320px] bg-foreground text-background p-3"
                  >
                    <p className="text-sm leading-relaxed">
                      {t(
                        "dictionary.aliases_help_tooltip",
                        "Add alternate spoken forms. Press Enter or comma to add. For symbols, add spoken aliases like c plus plus for c++.",
                      )}
                    </p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
            <div className="relative">
              <Input
                id="aliases"
                value={aliasDraft}
                onChange={(e) => setAliasDraft(e.target.value)}
                onKeyDown={handleAliasKeyDown}
                spellCheck={false}
                autoCorrect="off"
                autoCapitalize="none"
                autoComplete="off"
                disabled={aliases.length >= MAX_ALIASES_COUNT}
                onBlur={() => {
                  if (aliasDraft.trim()) {
                    addAliasesFromRaw(aliasDraft);
                    setAliasDraft("");
                  }
                }}
                onPaste={(e) => {
                  const pasted = e.clipboardData.getData("text");
                  if (/[,\n]/.test(pasted)) {
                    e.preventDefault();
                    addAliasesFromRaw(pasted);
                    setAliasDraft("");
                  }
                }}
                maxLength={ALIAS_MAX_LENGTH}
                placeholder={t(
                  isReplacement
                    ? "dictionary.aliases_placeholder_replacement"
                    : "dictionary.aliases_placeholder",
                  isReplacement
                    ? "Type alternate phrase and press Enter or comma (e.g. email address)"
                    : "Type alias and press Enter or comma (e.g. shad cn)",
                )}
                className="h-9 pr-10"
              />
              <button
                type="button"
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  addAliasesFromRaw(aliasDraft);
                  setAliasDraft("");
                }}
                disabled={
                  !aliasDraft.trim() || aliases.length >= MAX_ALIASES_COUNT
                }
                aria-label={t("dictionary.aliases_add", "Add")}
                className="absolute right-1.5 top-1/2 inline-flex h-6 w-6 -translate-y-1/2 items-center justify-center rounded-md text-muted-foreground/80 transition-colors hover:bg-muted hover:text-foreground disabled:opacity-40 disabled:cursor-not-allowed"
              >
                <Plus className="h-3.5 w-3.5" />
              </button>
            </div>
            {aliases.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {aliases.map((alias) => (
                  <button
                    type="button"
                    key={alias}
                    onClick={() => {
                      setAliases((prev) =>
                        prev.filter((item) => item !== alias),
                      );
                    }}
                    className="inline-flex items-center gap-1 rounded-full border border-border/70 bg-muted/50 px-2 py-1 text-xs text-foreground/90 hover:bg-muted"
                  >
                    {alias}
                    <X className="h-2.5 w-2.5 text-muted-foreground" />
                  </button>
                ))}
              </div>
            )}
            {aliases.length === 0 && (
              <p className="text-xs text-muted-foreground/70">
                {t(
                  isReplacement
                    ? "dictionary.aliases_empty_state_replacement"
                    : "dictionary.aliases_empty_state",
                  isReplacement
                    ? "No alternate phrases yet."
                    : "No aliases yet.",
                )}
              </p>
            )}
          </div>
        </div>

        <DialogFooter className="gap-3 sm:gap-2">
          <Button variant="outline" onClick={onClose} className="min-w-[5rem]">
            {t("common.cancel", "Cancel")}
          </Button>
          <Button
            onClick={handleSave}
            disabled={!canSave()}
            className="min-w-[6rem] bg-foreground text-background hover:bg-foreground/90"
          >
            {initialEntry
              ? t("common.save", "Save")
              : t("dictionary.add_word", "Add word")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
