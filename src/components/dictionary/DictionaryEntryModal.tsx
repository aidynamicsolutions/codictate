import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { ArrowRight, Check, Info, X, Plus } from "lucide-react";
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
import {
  deriveIntentFromEntry,
  DictionaryEntryIntent,
  isDuplicateEntry,
  isReplacementOutputValid,
  isShortSingleWordFuzzyBlocked,
  normalizeAliases,
} from "@/utils/dictionaryUtils";

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
  maxRows = 3,
  autoFocus,
  id,
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  onKeyDown?: (e: React.KeyboardEvent) => void;
  placeholder?: string;
  maxLength: number;
  maxRows?: number;
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
    const maxVisibleRows = maxRows;
    const minHeight = lineHeight * minRows + 16; // 16px for padding
    const maxHeight = lineHeight * maxVisibleRows + 16;

    // Set height based on content, clamped to min/max
    const newHeight = Math.min(
      Math.max(textarea.scrollHeight, minHeight),
      maxHeight,
    );
    textarea.style.height = `${newHeight}px`;
  }, [maxRows]);

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
  const recognizeIntentRef = useRef<HTMLButtonElement>(null);
  const replaceIntentRef = useRef<HTMLButtonElement>(null);
  const [intent, setIntent] = useState<DictionaryEntryIntent>("recognize");
  const [input, setInput] = useState("");
  const [replacement, setReplacement] = useState("");
  const [aliases, setAliases] = useState<string[]>([]);
  const [aliasDraft, setAliasDraft] = useState("");
  const [fuzzyEnabled, setFuzzyEnabled] = useState(false);
  const [duplicateError, setDuplicateError] = useState(false);
  const [showReplacementEqualityError, setShowReplacementEqualityError] =
    useState(false);

  const trimmedInput = useMemo(() => input.trim(), [input]);
  const trimmedReplacement = useMemo(() => replacement.trim(), [replacement]);
  const isReplacementMode = intent === "replace";
  const hasReplacementOutput = trimmedReplacement.length > 0;
  const hasValidReplacementOutput = useMemo(
    () => isReplacementOutputValid(input, replacement),
    [input, replacement],
  );

  const isFuzzyBlockedByShortTarget = useMemo(() => {
    if (isReplacementMode) {
      return false;
    }
    return isShortSingleWordFuzzyBlocked(input);
  }, [input, isReplacementMode]);
  const shouldShowFuzzyToggle = useMemo(() => {
    if (isReplacementMode) {
      return false;
    }
    if (!trimmedInput) {
      return false;
    }
    return !isFuzzyBlockedByShortTarget;
  }, [isReplacementMode, trimmedInput, isFuzzyBlockedByShortTarget]);

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
        const initialIntent = deriveIntentFromEntry(initialEntry);
        setInput(initialEntry.input);
        setIntent(initialIntent);
        setReplacement(initialIntent === "replace" ? initialEntry.replacement : "");
        setAliases(
          normalizeAliases(initialEntry.aliases ?? [], initialEntry.input),
        );
        setAliasDraft("");
        setFuzzyEnabled(
          initialIntent === "recognize" && initialEntry.fuzzy_enabled === true,
        );
        setShowReplacementEqualityError(false);
      } else {
        setIntent("recognize");
        setInput("");
        setReplacement("");
        setAliases([]);
        setAliasDraft("");
        setFuzzyEnabled(false);
        setShowReplacementEqualityError(false);
      }
    }
  }, [isOpen, initialEntry]);

  useEffect(() => {
    if (isFuzzyBlockedByShortTarget) {
      setFuzzyEnabled(false);
    }
  }, [isFuzzyBlockedByShortTarget]);

  useEffect(() => {
    if (!isReplacementMode || !hasReplacementOutput || hasValidReplacementOutput) {
      setShowReplacementEqualityError(false);
    }
  }, [isReplacementMode, hasReplacementOutput, hasValidReplacementOutput]);

  const canSave = useCallback(() => {
    if (!trimmedInput) return false;
    if (duplicateError) return false;
    if (isReplacementMode && !hasReplacementOutput) return false;
    return true;
  }, [trimmedInput, duplicateError, isReplacementMode, hasReplacementOutput]);

  const handleSave = useCallback(() => {
    if (!canSave()) return;
    if (isReplacementMode && !hasValidReplacementOutput) {
      setShowReplacementEqualityError(true);
      return;
    }

    const enforceExactOnly = isReplacementMode || isFuzzyBlockedByShortTarget;
    onSave({
      input: trimmedInput,
      aliases,
      replacement: isReplacementMode ? trimmedReplacement : trimmedInput,
      is_replacement: isReplacementMode,
      fuzzy_enabled: enforceExactOnly ? false : fuzzyEnabled,
    });
    onClose();
  }, [
    trimmedInput,
    trimmedReplacement,
    aliases,
    isReplacementMode,
    fuzzyEnabled,
    isFuzzyBlockedByShortTarget,
    hasValidReplacementOutput,
    canSave,
    onSave,
    onClose,
  ]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      // Preserve Enter for multi-line input; save with Cmd/Ctrl+Enter.
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter" && canSave()) {
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

  const handleIntentKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLButtonElement>) => {
      const focusIntentButton = (nextIntent: DictionaryEntryIntent) => {
        if (nextIntent === "recognize") {
          recognizeIntentRef.current?.focus();
        } else {
          replaceIntentRef.current?.focus();
        }
      };

      if (
        e.key === "ArrowRight" ||
        e.key === "ArrowDown" ||
        e.key === "ArrowLeft" ||
        e.key === "ArrowUp"
      ) {
        e.preventDefault();
        const orderedIntents: DictionaryEntryIntent[] = [
          "recognize",
          "replace",
        ];
        const currentIndex = orderedIntents.indexOf(intent);
        const direction =
          e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : -1;
        const nextIndex =
          (currentIndex + direction + orderedIntents.length) %
          orderedIntents.length;
        const nextIntent = orderedIntents[nextIndex];
        setIntent(nextIntent);
        focusIntentButton(nextIntent);
      } else if (e.key === "Home") {
        e.preventDefault();
        setIntent("recognize");
        focusIntentButton("recognize");
      } else if (e.key === "End") {
        e.preventDefault();
        setIntent("replace");
        focusIntentButton("replace");
      }
    },
    [intent],
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
          <div className="space-y-2">
            <Label className="text-sm font-medium">
              {t("dictionary.intent_label", "Entry intent")}
            </Label>
            <div
              role="radiogroup"
              aria-label={t("dictionary.intent_group_aria", "Entry intent")}
              className="grid gap-2 sm:grid-cols-2"
            >
              <button
                type="button"
                role="radio"
                ref={recognizeIntentRef}
                aria-checked={intent === "recognize"}
                tabIndex={intent === "recognize" ? 0 : -1}
                onClick={() => setIntent("recognize")}
                onKeyDown={handleIntentKeyDown}
                className={cn(
                  "rounded-xl border px-3 py-2.5 text-left transition-colors",
                  intent === "recognize"
                    ? "border-primary/50 bg-primary/10"
                    : "border-border/60 bg-muted/20 hover:bg-muted/35",
                )}
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <p className="text-sm font-medium">
                      {t(
                        "dictionary.intent_recognize_title",
                        "Recognize this term",
                      )}
                    </p>
                  </div>
                  {intent === "recognize" && (
                    <Check className="h-4 w-4 shrink-0 text-primary" />
                  )}
                </div>
              </button>

              <button
                type="button"
                role="radio"
                ref={replaceIntentRef}
                aria-checked={intent === "replace"}
                tabIndex={intent === "replace" ? 0 : -1}
                onClick={() => setIntent("replace")}
                onKeyDown={handleIntentKeyDown}
                className={cn(
                  "rounded-xl border px-3 py-2.5 text-left transition-colors",
                  intent === "replace"
                    ? "border-primary/50 bg-primary/10"
                    : "border-border/60 bg-muted/20 hover:bg-muted/35",
                )}
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <p className="text-sm font-medium">
                      {t(
                        "dictionary.intent_replace_title",
                        "Replace spoken phrase",
                      )}
                    </p>
                  </div>
                  {intent === "replace" && (
                    <Check className="h-4 w-4 shrink-0 text-primary" />
                  )}
                </div>
              </button>
            </div>
            <p className="text-xs text-muted-foreground">
              {intent === "recognize"
                ? t(
                    "dictionary.intent_selected_helper_recognize",
                    "Match this term exactly, with aliases.",
                  )
                : t(
                    "dictionary.intent_selected_helper_replace",
                    "When you say this, output different text.",
                  )}
            </p>
          </div>

          {isReplacementMode ? (
            <TooltipProvider delayDuration={500}>
              <div className="space-y-2">
                <div className="grid grid-cols-1 gap-2 md:grid-cols-[1fr_auto_1fr] md:gap-x-2 md:gap-y-1 md:items-start">
                  <div className="space-y-1 md:contents">
                    <Label htmlFor="input" className="text-sm font-medium md:col-start-1 md:row-start-1">
                      {t("dictionary.what_you_say_label", "What you say")}
                    </Label>
                    <div className="md:col-start-1 md:row-start-2">
                      <AutoGrowTextarea
                        id="input"
                        autoFocus
                        value={input}
                        onChange={setInput}
                        onKeyDown={handleKeyDown}
                        placeholder={t(
                          "dictionary.input_placeholder_replace",
                          "e.g. btw",
                        )}
                        maxLength={INPUT_MAX_LENGTH}
                        maxRows={4}
                        className={
                          duplicateError
                            ? "border-destructive focus-visible:ring-destructive"
                            : undefined
                        }
                      />
                    </div>
                  </div>

                  <div className="flex items-center justify-center pt-0.5 md:pt-2 md:col-start-2 md:row-start-2 md:self-start">
                    <ArrowRight className="h-4 w-4 rotate-90 text-muted-foreground/70 md:rotate-0" />
                  </div>

                  <div className="space-y-1 md:contents">
                    <div className="flex items-center gap-2 md:col-start-3 md:row-start-1">
                      <Label
                        htmlFor="replacement"
                        className="text-sm font-medium"
                      >
                        {t("dictionary.output_text_label", "Output text")}
                      </Label>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <button
                            type="button"
                            tabIndex={-1}
                            className="text-muted-foreground/60 hover:text-muted-foreground transition-colors"
                            aria-label={t(
                              "dictionary.replacement_help_tooltip_label",
                              "Replacement tips",
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
                              "dictionary.replacement_help_tooltip",
                              "Exact-only mapping. Add aliases for alternate spoken forms that should trigger this output.",
                            )}
                          </p>
                        </TooltipContent>
                      </Tooltip>
                    </div>
                    <div className="md:col-start-3 md:row-start-2">
                      <AutoGrowTextarea
                        id="replacement"
                        value={replacement}
                        onChange={setReplacement}
                        onKeyDown={handleKeyDown}
                        placeholder={t(
                          "dictionary.replacement_example",
                          "e.g. by the way",
                        )}
                        maxLength={REPLACEMENT_MAX_LENGTH}
                        maxRows={4}
                      />
                    </div>
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
                {showReplacementEqualityError && (
                  <p className="text-xs text-destructive">
                    {t(
                      "dictionary.replacement_inline_error",
                      "Output text must be different from what you say.",
                    )}
                  </p>
                )}
              </div>
            </TooltipProvider>
          ) : (
            <div className="space-y-2">
              <Label htmlFor="input" className="text-sm font-medium">
                {t("dictionary.word_or_phrase_label", "Word or phrase")}
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
                {isReplacementMode
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
                        "Aliases are exact alternatives that trigger this entry.",
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
                  isReplacementMode
                    ? "dictionary.aliases_placeholder_replacement"
                    : "dictionary.aliases_placeholder",
                  isReplacementMode
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
                disabled={!aliasDraft.trim() || aliases.length >= MAX_ALIASES_COUNT}
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
                      setAliases((prev) => prev.filter((item) => item !== alias));
                    }}
                    className="inline-flex items-center gap-1 rounded-full border border-border/70 bg-muted/50 px-2 py-1 text-xs text-foreground/90 hover:bg-muted"
                  >
                    {alias}
                    <X className="h-2.5 w-2.5 text-muted-foreground" />
                  </button>
                ))}
              </div>
            )}
          </div>

          {shouldShowFuzzyToggle && (
            <TooltipProvider delayDuration={500}>
              <div className="rounded-lg border border-border/60 bg-muted/20 px-3 py-2.5">
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2">
                    <Label
                      htmlFor="fuzzy-enabled"
                      className="text-sm font-medium cursor-pointer"
                    >
                      {t("dictionary.fuzzy_opt_in_label", "Enable fuzzy fallback")}
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
                            "dictionary.fuzzy_opt_in_tooltip",
                            "Use fuzzy only after exact input and aliases fail for uncommon long terms.",
                          )}
                        </p>
                      </TooltipContent>
                    </Tooltip>
                  </div>
                  <Switch
                    id="fuzzy-enabled"
                    checked={fuzzyEnabled}
                    onCheckedChange={setFuzzyEnabled}
                  />
                </div>
              </div>
            </TooltipProvider>
          )}
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
