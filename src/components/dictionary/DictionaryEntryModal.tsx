import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Info, ArrowRight } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { Button } from "@/components/shared/ui/button";
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
import { isDuplicateEntry } from "@/utils/dictionaryUtils";

// Character limits based on UX best practices:
// - Input (trigger phrase): 100 chars (~10-15 words) - longer phrases are unlikely to be consistently spoken
// - Replacement: 300 chars - more flexibility for expanded text, addresses, email signatures, etc.
const INPUT_MAX_LENGTH = 100;
const REPLACEMENT_MAX_LENGTH = 300;

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
    const newHeight = Math.min(Math.max(textarea.scrollHeight, minHeight), maxHeight);
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
        maxLength={maxLength}
        className={cn(
          "flex w-full rounded-xl border border-input bg-transparent px-3 py-2 text-sm shadow-xs transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 resize-none overflow-hidden",
          isAtLimit && "border-amber-500/50 focus-visible:ring-amber-500/50",
          isNearLimit && "pb-5", // Extra bottom padding when counter is visible
          className
        )}
        rows={1}
        style={{ minHeight: "36px" }}
      />
      {/* Character counter - shows when nearing or at limit */}
      {isNearLimit && (
        <span
          className={cn(
            "absolute right-2 bottom-1.5 text-[10px] font-medium",
            isAtLimit ? "text-amber-500" : "text-muted-foreground/50"
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
  const [isReplacement, setIsReplacement] = useState(false);
  const [duplicateError, setDuplicateError] = useState(false);

  // Check for duplicates when input changes
  useEffect(() => {
    if (!input.trim()) {
      setDuplicateError(false);
      return;
    }
    setDuplicateError(isDuplicateEntry(input, existingEntries, initialEntry));
  }, [input, existingEntries, initialEntry]);

  useEffect(() => {
    if (isOpen) {
      if (initialEntry) {
        setInput(initialEntry.input);
        setReplacement(initialEntry.replacement);
        setIsReplacement(initialEntry.is_replacement);
      } else {
        setInput("");
        setReplacement("");
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
      // When replacement mode is on, use custom replacement; otherwise same as input
      replacement: isReplacement ? replacement.trim() : input.trim(),
      is_replacement: isReplacement,
    });
    onClose();
  }, [input, replacement, isReplacement, canSave, onSave, onClose]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      // Save on Enter (since we're using single-line style now) or Cmd/Ctrl + Enter
      if (e.key === "Enter" && !e.shiftKey && canSave()) {
        e.preventDefault();
        handleSave();
      }
    },
    [canSave, handleSave]
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
              "Add a word or phrase you want Codictate to recognize."
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
                  {t("dictionary.replace_with_text", "Replace with different text")}
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
                        "Enable this to replace phrases with different text. Examples: 'chat gpt' → 'ChatGPT', 'btw' → 'by the way', 'my email' → 'john@example.com'. Tip: match the word count (e.g. 'chat gpt' is 2 words, so it matches when you say 2 words)."
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
                  <Label htmlFor="input" className="text-xs font-medium text-muted-foreground">
                    {t("dictionary.when_i_say", "When I say...")}
                  </Label>
                  <AutoGrowTextarea
                    id="input"
                    autoFocus
                    value={input}
                    onChange={setInput}
                    onKeyDown={handleKeyDown}
                    className={duplicateError ? "border-destructive focus-visible:ring-destructive" : undefined}
                    placeholder={t("dictionary.input_example", "e.g. my email")}
                    maxLength={INPUT_MAX_LENGTH}
                  />
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0 mt-7" />
                <div className="flex-1 space-y-1.5">
                  <Label htmlFor="replacement" className="text-xs font-medium text-muted-foreground">
                    {t("dictionary.replace_with", "Replace with...")}
                  </Label>
                  <AutoGrowTextarea
                    id="replacement"
                    value={replacement}
                    onChange={setReplacement}
                    onKeyDown={handleKeyDown}
                    placeholder={t("dictionary.replacement_example", "e.g. john@example.com")}
                    maxLength={REPLACEMENT_MAX_LENGTH}
                  />
                </div>
              </div>
              {duplicateError && (
                <p className="text-xs text-destructive">
                  {t("dictionary.duplicate_error", "This word already exists in your dictionary")}
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
                placeholder={t("dictionary.input_placeholder", "e.g. Codictate, EBITDA, John Smith")}
                maxLength={INPUT_MAX_LENGTH}
                className={duplicateError ? "border-destructive focus-visible:ring-destructive" : undefined}
              />
              {duplicateError && (
                <p className="text-xs text-destructive">
                  {t("dictionary.duplicate_error", "This word already exists in your dictionary")}
                </p>
              )}
            </div>
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
