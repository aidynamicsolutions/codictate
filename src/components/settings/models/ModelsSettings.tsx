import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask } from "@tauri-apps/plugin-dialog";
import { ChevronDown, Globe, Info } from "lucide-react";
import type { ModelCardStatus } from "@/components/onboarding";
import { ModelCard } from "@/components/onboarding";
import { LlmModelCard, type LlmModelCardStatus } from "@/components/onboarding/LlmModelCard";
import { useModelStore } from "@/stores/modelStore";
import { useMlxModels } from "@/hooks/useMlxModels";
import { useSettings } from "@/hooks/useSettings";
import { LANGUAGES } from "@/lib/constants/languages.ts";
import type { ModelInfo, MlxModelInfo } from "@/bindings";
import { commands } from "@/bindings";
import { cn } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/shared/ui/collapsible";

// check if model supports a language based on its supported_languages list
const modelSupportsLanguage = (model: ModelInfo, langCode: string): boolean => {
  return model.supported_languages.includes(langCode);
};

export const ModelsSettings: React.FC = () => {
  const { t } = useTranslation();
  const [switchingModelId, setSwitchingModelId] = useState<string | null>(null);
  const [languageFilter, setLanguageFilter] = useState("all");
  const [languageDropdownOpen, setLanguageDropdownOpen] = useState(false);
  const [languageSearch, setLanguageSearch] = useState("");
  const languageDropdownRef = useRef<HTMLDivElement>(null);
  const languageSearchInputRef = useRef<HTMLInputElement>(null);
  const languageModelsSectionRef = useRef<HTMLDivElement>(null);
  
  // Refs for scroll-to-section on collapse
  const downloadedModelsSectionRef = useRef<HTMLDivElement>(null);
  const availableModelsSectionRef = useRef<HTMLDivElement>(null);
  const downloadedMlxSectionRef = useRef<HTMLDivElement>(null);
  const availableMlxSectionRef = useRef<HTMLDivElement>(null);
  
  // Collapsible state for model sections
  const [downloadedModelsExpanded, setDownloadedModelsExpanded] = useState(false);
  const [availableModelsExpanded, setAvailableModelsExpanded] = useState(false);
  const [languageModelsExpanded, setLanguageModelsExpanded] = useState(false);
  const PREVIEW_COUNT = 2; // Number of available models to show when collapsed
  
  // Smart expand/collapse handlers that scroll to section on collapse
  const handleDownloadedModelsToggle = (open: boolean) => {
    setDownloadedModelsExpanded(open);
    if (!open && downloadedModelsSectionRef.current) {
      // Small delay to let animation start before scrolling
      setTimeout(() => {
        downloadedModelsSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }, 50);
    }
  };
  
  const handleAvailableModelsToggle = (open: boolean) => {
    setAvailableModelsExpanded(open);
    if (!open && availableModelsSectionRef.current) {
      setTimeout(() => {
        availableModelsSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }, 50);
    }
  };
  
  const handleLanguageModelsToggle = (open: boolean) => {
    setLanguageModelsExpanded(open);
    if (!open && languageModelsSectionRef.current) {
      setTimeout(() => {
        languageModelsSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }, 50);
    }
  };
  const {
    models,
    currentModel,
    downloadingModels,
    downloadProgress,
    downloadStats,
    extractingModels,
    loading,
    downloadModel,
    cancelDownload,
    selectModel,
    deleteModel,
    shouldScrollToLanguageModels,
    setShouldScrollToLanguageModels,
  } = useModelStore();

  // MLX Language Models state
  const {
    models: mlxModels,
    isLoading: mlxLoading,
    downloadProgress: mlxDownloadProgress,
    downloadingModelId,
    downloadModel: mlxDownloadModel,
    cancelDownload: mlxCancelDownload,
    retryDownload: mlxRetryDownload,
    deleteModel: mlxDeleteModel,
    selectModel: mlxSelectModel,
  } = useMlxModels();
  const { settings, updatePostProcessModel } = useSettings();
  const selectedMlxModelId = settings?.post_process_models?.local_mlx ?? null;

  // click outside handler for language dropdown
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        languageDropdownRef.current &&
        !languageDropdownRef.current.contains(event.target as Node)
      ) {
        setLanguageDropdownOpen(false);
        setLanguageSearch("");
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  // focus search input when dropdown opens
  useEffect(() => {
    if (languageDropdownOpen && languageSearchInputRef.current) {
      languageSearchInputRef.current.focus();
    }
  }, [languageDropdownOpen]);

  // Scroll to Language Models section when navigating from Post Processing
  useEffect(() => {
    if (shouldScrollToLanguageModels && languageModelsSectionRef.current) {
      // Small delay to ensure DOM is rendered
      setTimeout(() => {
        languageModelsSectionRef.current?.scrollIntoView({
          behavior: "smooth",
          block: "start",
        });
        setShouldScrollToLanguageModels(false);
      }, 100);
    }
  }, [shouldScrollToLanguageModels, setShouldScrollToLanguageModels]);

  // filtered languages for dropdown (exclude "auto")
  const filteredLanguages = useMemo(() => {
    return LANGUAGES.filter(
      (lang) =>
        lang.value !== "auto" &&
        lang.label.toLowerCase().includes(languageSearch.toLowerCase()),
    );
  }, [languageSearch]);

  // Get selected language label
  const selectedLanguageLabel = useMemo(() => {
    if (languageFilter === "all") {
      return t("settings.models.filters.allLanguages");
    }
    return LANGUAGES.find((lang) => lang.value === languageFilter)?.label || "";
  }, [languageFilter, t]);

  const getModelStatus = (modelId: string): ModelCardStatus => {
    if (modelId in extractingModels) {
      return "extracting";
    }
    if (modelId in downloadingModels) {
      return "downloading";
    }
    if (switchingModelId === modelId) {
      return "switching";
    }
    if (modelId === currentModel) {
      return "active";
    }
    const model = models.find((m: ModelInfo) => m.id === modelId);
    if (model?.is_downloaded) {
      return "available";
    }
    return "downloadable";
  };

  const getDownloadProgress = (modelId: string): number | undefined => {
    const progress = downloadProgress[modelId];
    return progress?.percentage;
  };

  const getDownloadSpeed = (modelId: string): number | undefined => {
    const stats = downloadStats[modelId];
    return stats?.speed;
  };

  const handleModelSelect = async (modelId: string) => {
    setSwitchingModelId(modelId);
    try {
      await selectModel(modelId);
    } finally {
      setSwitchingModelId(null);
    }
  };

  const handleModelDownload = async (modelId: string) => {
    await downloadModel(modelId);
  };

  const handleModelDelete = async (modelId: string) => {
    const model = models.find((m: ModelInfo) => m.id === modelId);
    const modelName = model?.name || modelId;
    const isActive = modelId === currentModel;

    const confirmed = await ask(
      isActive
        ? t("settings.models.deleteActiveConfirm", { modelName })
        : t("settings.models.deleteConfirm", { modelName }),
      {
        title: t("settings.models.deleteTitle"),
        kind: "warning",
      },
    );

    if (confirmed) {
      try {
        await deleteModel(modelId);
      } catch (err) {
        console.error(`Failed to delete model ${modelId}:`, err);
      }
    }
  };

  const handleModelCancel = async (modelId: string) => {
    try {
      await cancelDownload(modelId);
    } catch (err) {
      console.error(`Failed to cancel download for ${modelId}:`, err);
    }
  };

  // MLX Model handlers
  const getMlxModelStatus = (model: MlxModelInfo): LlmModelCardStatus => {
    if (model.status === "downloading") return "downloading";
    if (model.status === "download_failed") return "download_failed";
    if (model.status === "downloaded" || model.status === "loading" || model.status === "ready") {
      return selectedMlxModelId === model.id ? "active" : "available";
    }
    return "downloadable";
  };

  const handleMlxModelSelect = async (modelId: string) => {
    // Persist the model selection to settings
    await updatePostProcessModel("local_mlx", modelId);
    // Also switch the active model in the backend
    await mlxSelectModel(modelId);
  };

  const handleMlxModelDownload = async (modelId: string) => {
    await mlxDownloadModel(modelId);
  };

  const handleMlxModelDelete = async (modelId: string) => {
    const model = mlxModels.find((m: MlxModelInfo) => m.id === modelId);
    const modelName = model?.display_name || modelId;
    const isSelected = selectedMlxModelId === modelId;

    const confirmed = await ask(
      isSelected
        ? t("settings.models.deleteActiveConfirm", { modelName })
        : t("settings.models.deleteConfirm", { modelName }),
      {
        title: t("settings.models.deleteTitle"),
        kind: "warning",
      },
    );

    if (confirmed) {
      await mlxDeleteModel(modelId);
    }
  };

  const handleMlxModelCancel = async (_modelId: string) => {
    await mlxCancelDownload();
  };

  const handleMlxModelRetry = async (_modelId: string) => {
    await mlxRetryDownload();
  };

  const handleMlxShowInFinder = async (modelId: string) => {
    await commands.mlxOpenModelsDir(modelId);
  };

  // Filter models based on language filter
  const filteredModels = useMemo(() => {
    return models.filter((model: ModelInfo) => {
      if (languageFilter !== "all") {
        if (!modelSupportsLanguage(model, languageFilter)) return false;
      }
      return true;
    });
  }, [models, languageFilter]);

  // Split filtered models into downloaded and available sections
  const { downloadedModels, availableModels } = useMemo(() => {
    const downloaded: ModelInfo[] = [];
    const available: ModelInfo[] = [];

    for (const model of filteredModels) {
      const isDownloaded =
        model.is_downloaded ||
        model.id in downloadingModels ||
        model.id in extractingModels;
      if (isDownloaded) {
        downloaded.push(model);
      } else {
        available.push(model);
      }
    }

    // Sort downloaded models so the active model is always first
    downloaded.sort((a, b) => {
      if (a.id === currentModel) return -1;
      if (b.id === currentModel) return 1;
      return 0;
    });

    return { downloadedModels: downloaded, availableModels: available };
  }, [filteredModels, downloadingModels, extractingModels, currentModel]);

  // Split MLX models into downloaded and available sections
  const { downloadedMlxModels, availableMlxModels } = useMemo(() => {
    const downloaded: MlxModelInfo[] = [];
    const available: MlxModelInfo[] = [];

    for (const model of mlxModels) {
      const isDownloaded =
        model.status === "downloaded" ||
        model.status === "loading" ||
        model.status === "ready" ||
        model.status === "downloading";
      if (isDownloaded) {
        downloaded.push(model);
      } else {
        available.push(model);
      }
    }

    // Sort downloaded models so the selected model is always first
    downloaded.sort((a, b) => {
      if (a.id === selectedMlxModelId) return -1;
      if (b.id === selectedMlxModelId) return 1;
      if (a.is_default && !b.is_default) return -1;
      if (!a.is_default && b.is_default) return 1;
      return 0;
    });

    return { downloadedMlxModels: downloaded, availableMlxModels: available };
  }, [mlxModels, selectedMlxModelId]);

  if (loading) {
    return (
      <div className="max-w-3xl w-full mx-auto">
        <div className="flex items-center justify-center py-16">
          <div className="w-8 h-8 border-2 border-logo-primary border-t-transparent rounded-full animate-spin" />
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-3xl w-full mx-auto space-y-4">
      <div className="mb-4">
        <h1 className="text-xl font-semibold mb-2">
          {t("settings.models.title")}
        </h1>
        <p className="text-sm text-text/60">
          {t("settings.models.description")}
        </p>
      </div>
      {filteredModels.length > 0 ? (
        <div className="space-y-6">
          {/* Downloaded Models Section */}
          {downloadedModels.length > 0 && (
            <div className="space-y-3" ref={downloadedModelsSectionRef}>
              <div className="flex items-center justify-between">
                <h2 className="text-sm font-medium text-text/60">
                  {t("settings.models.yourModels")}
                </h2>
                {/* Language filter dropdown */}
                <div className="relative" ref={languageDropdownRef}>
                  <button
                    type="button"
                    onClick={() =>
                      setLanguageDropdownOpen(!languageDropdownOpen)
                    }
                    className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                      languageFilter !== "all"
                        ? "bg-logo-primary/20 text-logo-primary"
                        : "bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20"
                    }`}
                  >
                    <Globe className="w-3.5 h-3.5" />
                    <span className="max-w-[120px] truncate">
                      {selectedLanguageLabel}
                    </span>
                    <ChevronDown
                      className={`w-3.5 h-3.5 transition-transform ${
                        languageDropdownOpen ? "rotate-180" : ""
                      }`}
                    />
                  </button>

                  {languageDropdownOpen && (
                    <div className="absolute top-full right-0 mt-1 w-56 bg-background border border-mid-gray/80 rounded-lg shadow-lg z-50 overflow-hidden">
                      <div className="p-2 border-b border-mid-gray/40">
                        <input
                          ref={languageSearchInputRef}
                          type="text"
                          value={languageSearch}
                          onChange={(e) => setLanguageSearch(e.target.value)}
                          onKeyDown={(e) => {
                            if (
                              e.key === "Enter" &&
                              filteredLanguages.length > 0
                            ) {
                              setLanguageFilter(filteredLanguages[0].value);
                              setLanguageDropdownOpen(false);
                              setLanguageSearch("");
                            } else if (e.key === "Escape") {
                              setLanguageDropdownOpen(false);
                              setLanguageSearch("");
                            }
                          }}
                          placeholder={t(
                            "settings.general.language.searchPlaceholder",
                          )}
                          className="w-full px-2 py-1 text-sm bg-mid-gray/10 border border-mid-gray/40 rounded-md focus:outline-none focus:ring-1 focus:ring-logo-primary"
                        />
                      </div>
                      <div className="max-h-48 overflow-y-auto">
                        <button
                          type="button"
                          onClick={() => {
                            setLanguageFilter("all");
                            setLanguageDropdownOpen(false);
                            setLanguageSearch("");
                          }}
                          className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                            languageFilter === "all"
                              ? "bg-logo-primary/20 text-logo-primary font-semibold"
                              : "hover:bg-mid-gray/10"
                          }`}
                        >
                          {t("settings.models.filters.allLanguages")}
                        </button>
                        {filteredLanguages.map((lang) => (
                          <button
                            key={lang.value}
                            type="button"
                            onClick={() => {
                              setLanguageFilter(lang.value);
                              setLanguageDropdownOpen(false);
                              setLanguageSearch("");
                            }}
                            className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                              languageFilter === lang.value
                                ? "bg-logo-primary/20 text-logo-primary font-semibold"
                                : "hover:bg-mid-gray/10"
                            }`}
                          >
                            {lang.label}
                          </button>
                        ))}
                        {filteredLanguages.length === 0 && (
                          <div className="px-3 py-2 text-sm text-text/50 text-center">
                            {t("settings.general.language.noResults")}
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>
              {/* Active model - always visible (first in sorted list) */}
              {downloadedModels.length > 0 && (
                <ModelCard
                  key={downloadedModels[0].id}
                  model={downloadedModels[0]}
                  status={getModelStatus(downloadedModels[0].id)}
                  onSelect={handleModelSelect}
                  onDownload={handleModelDownload}
                  onDelete={handleModelDelete}
                  onCancel={handleModelCancel}
                  downloadProgress={getDownloadProgress(downloadedModels[0].id)}
                  downloadSpeed={getDownloadSpeed(downloadedModels[0].id)}
                  showRecommended={false}
                />
              )}
              
              {/* Collapsible for other downloaded models */}
              {downloadedModels.length > 1 && (
                <Collapsible
                  open={downloadedModelsExpanded}
                  onOpenChange={handleDownloadedModelsToggle}
                >
                  <CollapsibleContent className="space-y-3">
                    {downloadedModels.slice(1).map((model: ModelInfo) => (
                      <ModelCard
                        key={model.id}
                        model={model}
                        status={getModelStatus(model.id)}
                        onSelect={handleModelSelect}
                        onDownload={handleModelDownload}
                        onDelete={handleModelDelete}
                        onCancel={handleModelCancel}
                        downloadProgress={getDownloadProgress(model.id)}
                        downloadSpeed={getDownloadSpeed(model.id)}
                        showRecommended={false}
                      />
                    ))}
                  </CollapsibleContent>
                  <CollapsibleTrigger asChild>
                    <button
                      className="flex items-center gap-2 w-full py-2 text-sm text-text/60 hover:text-text transition-colors"
                    >
                      <ChevronDown
                        className={cn(
                          "w-4 h-4 transition-transform duration-200",
                          downloadedModelsExpanded && "rotate-180"
                        )}
                      />
                      {downloadedModelsExpanded
                        ? t("settings.models.showLess")
                        : t("settings.models.showMore", { count: downloadedModels.length - 1 })}
                    </button>
                  </CollapsibleTrigger>
                </Collapsible>
              )}
            </div>
          )}

          {/* Available Models Section */}
          {availableModels.length > 0 && (
            <div className="space-y-3" ref={availableModelsSectionRef}>
              <h2 className="text-sm font-medium text-text/60">
                {t("settings.models.availableModels")}
              </h2>
              {/* Preview models - always visible */}
              {availableModels.slice(0, PREVIEW_COUNT).map((model: ModelInfo) => (
                <ModelCard
                  key={model.id}
                  model={model}
                  status={getModelStatus(model.id)}
                  onSelect={handleModelSelect}
                  onDownload={handleModelDownload}
                  onDelete={handleModelDelete}
                  onCancel={handleModelCancel}
                  downloadProgress={getDownloadProgress(model.id)}
                  downloadSpeed={getDownloadSpeed(model.id)}
                  showRecommended={false}
                />
              ))}
              
              {/* Collapsible hidden models */}
              {availableModels.length > PREVIEW_COUNT && (
                <Collapsible
                  open={availableModelsExpanded}
                  onOpenChange={handleAvailableModelsToggle}
                >
                  <CollapsibleContent className="space-y-3">
                    {availableModels.slice(PREVIEW_COUNT).map((model: ModelInfo) => (
                      <ModelCard
                        key={model.id}
                        model={model}
                        status={getModelStatus(model.id)}
                        onSelect={handleModelSelect}
                        onDownload={handleModelDownload}
                        onDelete={handleModelDelete}
                        onCancel={handleModelCancel}
                        downloadProgress={getDownloadProgress(model.id)}
                        downloadSpeed={getDownloadSpeed(model.id)}
                        showRecommended={false}
                      />
                    ))}
                  </CollapsibleContent>
                  <CollapsibleTrigger asChild>
                    <button
                      className="flex items-center gap-2 w-full py-2 text-sm text-text/60 hover:text-text transition-colors"
                    >
                      <ChevronDown
                        className={cn(
                          "w-4 h-4 transition-transform duration-200",
                          availableModelsExpanded && "rotate-180"
                        )}
                      />
                      {availableModelsExpanded
                        ? t("settings.models.showLess")
                        : t("settings.models.showMore", { count: availableModels.length - PREVIEW_COUNT })}
                    </button>
                  </CollapsibleTrigger>
                </Collapsible>
              )}
            </div>
          )}
        </div>
      ) : (
        <div className="text-center py-8 text-text/50">
          {t("settings.models.noModelsMatch")}
        </div>
      )}

      {/* Language Models Section */}
      <div
        ref={languageModelsSectionRef}
        id="language-models-section"
        className="mt-8 pt-6 border-t border-mid-gray/20"
      >
        <div className="mb-4">
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold">
              {t("settings.models.languageModels.title")}
            </h1>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button className="text-text/40 hover:text-text/60 transition-colors">
                    <Info className="w-4 h-4" />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right" className="max-w-xs">
                  <p>{t("settings.models.languageModels.infoTooltip")}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
          <p className="text-sm text-text/60 mt-2">
            {t("settings.models.languageModels.description")}
          </p>
        </div>

        {mlxLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="w-6 h-6 border-2 border-logo-primary border-t-transparent rounded-full animate-spin" />
          </div>
        ) : (
          <div className="space-y-6">
            {/* Downloaded Language Models */}
            {downloadedMlxModels.length > 0 && (
              <div className="space-y-3">
                <h2 className="text-sm font-medium text-text/60">
                  {t("settings.models.languageModels.yourModels")}
                </h2>
                {downloadedMlxModels.map((model: MlxModelInfo) => (
                  <LlmModelCard
                    key={model.id}
                    model={model}
                    status={getMlxModelStatus(model)}
                    isSelected={selectedMlxModelId === model.id}
                    downloadProgress={
                      downloadingModelId === model.id ? mlxDownloadProgress : null
                    }
                    onSelect={handleMlxModelSelect}
                    onDownload={handleMlxModelDownload}
                    onDelete={handleMlxModelDelete}
                    onCancel={handleMlxModelCancel}
                    onRetry={handleMlxModelRetry}
                    onShowInFinder={handleMlxShowInFinder}
                  />
                ))}
              </div>
            )}

            {/* Available Language Models */}
            {availableMlxModels.length > 0 && (
              <div className="space-y-3">
                <h2 className="text-sm font-medium text-text/60">
                  {t("settings.models.languageModels.availableModels")}
                </h2>
                {/* Preview models - always visible */}
                {availableMlxModels.slice(0, PREVIEW_COUNT).map((model: MlxModelInfo) => (
                  <LlmModelCard
                    key={model.id}
                    model={model}
                    status={getMlxModelStatus(model)}
                    isSelected={false}
                    downloadProgress={
                      downloadingModelId === model.id ? mlxDownloadProgress : null
                    }
                    onSelect={handleMlxModelSelect}
                    onDownload={handleMlxModelDownload}
                    onDelete={handleMlxModelDelete}
                    onCancel={handleMlxModelCancel}
                    onRetry={handleMlxModelRetry}
                    onShowInFinder={handleMlxShowInFinder}
                  />
                ))}
                
                {/* Collapsible hidden models */}
                {availableMlxModels.length > PREVIEW_COUNT && (
                  <Collapsible
                    open={languageModelsExpanded}
                    onOpenChange={handleLanguageModelsToggle}
                  >
                    <CollapsibleContent className="space-y-3">
                      {availableMlxModels.slice(PREVIEW_COUNT).map((model: MlxModelInfo) => (
                        <LlmModelCard
                          key={model.id}
                          model={model}
                          status={getMlxModelStatus(model)}
                          isSelected={false}
                          downloadProgress={
                            downloadingModelId === model.id ? mlxDownloadProgress : null
                          }
                          onSelect={handleMlxModelSelect}
                          onDownload={handleMlxModelDownload}
                          onDelete={handleMlxModelDelete}
                          onCancel={handleMlxModelCancel}
                          onRetry={handleMlxModelRetry}
                          onShowInFinder={handleMlxShowInFinder}
                        />
                      ))}
                    </CollapsibleContent>
                    <CollapsibleTrigger asChild>
                      <button
                        className="flex items-center gap-2 w-full py-2 text-sm text-text/60 hover:text-text transition-colors"
                      >
                        <ChevronDown
                          className={cn(
                            "w-4 h-4 transition-transform duration-200",
                            languageModelsExpanded && "rotate-180"
                          )}
                        />
                        {languageModelsExpanded
                          ? t("settings.models.showLess")
                          : t("settings.models.showMore", { count: availableMlxModels.length - PREVIEW_COUNT })}
                      </button>
                    </CollapsibleTrigger>
                  </Collapsible>
                )}
              </div>
            )}

            {mlxModels.length === 0 && (
              <div className="text-center py-8 text-text/50">
                {t("settings.models.languageModels.noModels")}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
