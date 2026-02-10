import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCcw } from "lucide-react";
import { commands } from "@/bindings";

import { Alert } from "../../ui/Alert";
import {
  Dropdown,
  SettingContainer,
  SettingsGroup,
} from "@/components/ui";
import { Textarea } from "@/components/shared/ui/textarea";
import { Button } from "@/components/shared/ui/button";
import { Input } from "@/components/shared/ui/input";
import { Switch } from "@/components/shared/ui/switch";

import { ProviderSelect } from "../PostProcessingSettingsApi/ProviderSelect";
import { BaseUrlField } from "../PostProcessingSettingsApi/BaseUrlField";
import { ApiKeyField } from "../PostProcessingSettingsApi/ApiKeyField";
import { ModelSelect } from "../PostProcessingSettingsApi/ModelSelect";
import { usePostProcessProviderState } from "../PostProcessingSettingsApi/usePostProcessProviderState";
import { ShortcutInput } from "../ShortcutInput";
import { useSettings } from "../../../hooks/useSettings";
import { useMlxModels } from "@/hooks/useMlxModels";
import { useModelStore } from "@/stores/modelStore";

interface PostProcessingSettingsApiProps {
  onNavigate?: (section: string) => void;
}

const PostProcessingSettingsApiComponent: React.FC<PostProcessingSettingsApiProps> = ({ onNavigate }) => {
  const { t } = useTranslation();
  const state = usePostProcessProviderState();
  const { models: mlxModels } = useMlxModels();
  const { setShouldScrollToLanguageModels } = useModelStore();

  // Find the selected MLX model's display name
  const selectedMlxModel = mlxModels.find((m) => m.id === state.model);
  const selectedMlxModelName = selectedMlxModel?.display_name ?? state.model;
  return (
    <>
      <SettingContainer
        title={t("settings.refine.api.provider.title")}
        description={t("settings.refine.api.provider.description")}
        descriptionMode="tooltip"
        layout="horizontal"
        grouped={true}
      >
        <div className="flex items-center gap-2">
          <ProviderSelect
            options={state.providerOptions}
            value={state.selectedProviderId}
            onChange={state.handleProviderSelect}
          />
        </div>
      </SettingContainer>

      {state.isAppleProvider ? (
        state.appleIntelligenceUnavailable ? (
          <Alert variant="error" contained>
            {t("settings.refine.api.appleIntelligence.unavailable")}
          </Alert>
        ) : null
      ) : state.isMlxProvider ? (
        <SettingContainer
          title={t("settings.refine.mlx.title")}
          description={t("settings.refine.mlx.description")}
          descriptionMode="tooltip"
          layout="horizontal"
          grouped={true}
        >
          <div className="flex items-center gap-2">
            {state.model ? (
              <>
                <span className="text-sm text-text">
                  {selectedMlxModelName}
                </span>
                <button
                  onClick={() => {
                    setShouldScrollToLanguageModels(true);
                    onNavigate?.("models");
                  }}
                  className="text-xs text-logo-primary hover:underline"
                >
                  ({t("settings.models.languageModels.viewInModels")})
                </button>
              </>
            ) : (
              <Button
                variant="default"
                size="sm"
                onClick={() => {
                  setShouldScrollToLanguageModels(true);
                  onNavigate?.("models");
                }}
              >
                {t("settings.refine.mlx.downloadModel")}
              </Button>
            )}
          </div>
        </SettingContainer>
      ) : (
        <>
          {state.selectedProvider?.id === "custom" && (
            <SettingContainer
              title={t("settings.refine.api.baseUrl.title")}
              description={t("settings.refine.api.baseUrl.description")}
              descriptionMode="tooltip"
              layout="horizontal"
              grouped={true}
            >
              <div className="flex items-center gap-2">
                <BaseUrlField
                  value={state.baseUrl}
                  onBlur={state.handleBaseUrlChange}
                  placeholder={t(
                    "settings.refine.api.baseUrl.placeholder",
                  )}
                  disabled={state.isBaseUrlUpdating}
                  className="min-w-[380px]"
                />
              </div>
            </SettingContainer>
          )}

          <SettingContainer
            title={t("settings.refine.api.apiKey.title")}
            description={t("settings.refine.api.apiKey.description")}
            descriptionMode="tooltip"
            layout="horizontal"
            grouped={true}
          >
            <div className="flex items-center gap-2">
              <ApiKeyField
                value={state.apiKey}
                onBlur={state.handleApiKeyChange}
                placeholder={t(
                  "settings.refine.api.apiKey.placeholder",
                )}
                disabled={state.isApiKeyUpdating}
                className="min-w-[320px]"
              />
            </div>
          </SettingContainer>
        </>
      )}

      {/* Hide model dropdown for Apple and MLX providers - MLX uses Models page */}
      {!state.isAppleProvider && !state.isMlxProvider && (
        <SettingContainer
          title={t("settings.refine.api.model.title")}
          description={
            state.isCustomProvider
              ? t("settings.refine.api.model.descriptionCustom")
              : t("settings.refine.api.model.descriptionDefault")
          }
          descriptionMode="tooltip"
          layout="stacked"
          grouped={true}
        >
          <div className="flex items-center gap-2">
            <ModelSelect
              value={state.model}
              options={state.modelOptions}
              disabled={state.isModelUpdating}
              isLoading={state.isFetchingModels}
              placeholder={
                state.modelOptions.length > 0
                  ? t(
                      "settings.refine.api.model.placeholderWithOptions",
                    )
                  : t("settings.refine.api.model.placeholderNoOptions")
              }
              onSelect={state.handleModelSelect}
              onCreate={state.handleModelCreate}
              onBlur={() => {}}
              className="flex-1 min-w-[380px]"
            />
            <Button
              onClick={state.handleRefreshModels}
              disabled={state.isFetchingModels}
              title={t("settings.refine.api.model.refreshModels")}
              variant="ghost"
              size="icon"
              className="h-10 w-10"
            >
              <RefreshCcw
                className={`h-4 w-4 ${state.isFetchingModels ? "animate-spin" : ""}`}
              />
            </Button>
          </div>
        </SettingContainer>
      )}
    </>
  );
};

const PostProcessingSettingsPromptsComponent: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating, refreshSettings } =
    useSettings();
  const [isCreating, setIsCreating] = useState(false);
  const [draftName, setDraftName] = useState("");
  const [draftText, setDraftText] = useState("");

  const prompts = getSetting("post_process_prompts") || [];
  const selectedPromptId = getSetting("post_process_selected_prompt_id") || "";
  const selectedPrompt =
    prompts.find((prompt) => prompt.id === selectedPromptId) || null;

  useEffect(() => {
    if (isCreating) return;

    if (selectedPrompt) {
      setDraftName(selectedPrompt.name);
      setDraftText(selectedPrompt.prompt);
    } else {
      setDraftName("");
      setDraftText("");
    }
  }, [
    isCreating,
    selectedPromptId,
    selectedPrompt?.name,
    selectedPrompt?.prompt,
  ]);

  const handlePromptSelect = (promptId: string | null) => {
    if (!promptId) return;
    updateSetting("post_process_selected_prompt_id", promptId);
    setIsCreating(false);
  };

  const handleCreatePrompt = async () => {
    if (!draftName.trim() || !draftText.trim()) return;

    try {
      const result = await commands.addPostProcessPrompt(
        draftName.trim(),
        draftText.trim(),
      );
      if (result.status === "ok") {
        await refreshSettings();
        updateSetting("post_process_selected_prompt_id", result.data.id);
        setIsCreating(false);
      }
    } catch (error) {
      console.error("Failed to create prompt:", error);
    }
  };

  const handleUpdatePrompt = async () => {
    if (!selectedPromptId || !draftName.trim() || !draftText.trim()) return;

    try {
      await commands.updatePostProcessPrompt(
        selectedPromptId,
        draftName.trim(),
        draftText.trim(),
      );
      await refreshSettings();
    } catch (error) {
      console.error("Failed to update prompt:", error);
    }
  };

  const handleDeletePrompt = async (promptId: string) => {
    if (!promptId) return;

    try {
      await commands.deletePostProcessPrompt(promptId);
      await refreshSettings();
      setIsCreating(false);
    } catch (error) {
      console.error("Failed to delete prompt:", error);
    }
  };

  const handleCancelCreate = () => {
    setIsCreating(false);
    if (selectedPrompt) {
      setDraftName(selectedPrompt.name);
      setDraftText(selectedPrompt.prompt);
    } else {
      setDraftName("");
      setDraftText("");
    }
  };

  const handleStartCreate = () => {
    setIsCreating(true);
    setDraftName("");
    setDraftText("");
  };

  const hasPrompts = prompts.length > 0;
  const isDirty =
    !!selectedPrompt &&
    (draftName.trim() !== selectedPrompt.name ||
      draftText.trim() !== selectedPrompt.prompt.trim());

  return (
    <SettingContainer
      title={t("settings.refine.prompts.selectedPrompt.title")}
      description={t(
        "settings.refine.prompts.selectedPrompt.description",
      )}
      descriptionMode="tooltip"
      layout="stacked"
      grouped={true}
    >
      <div className="space-y-3">
        <div className="flex gap-2">
          <Dropdown
            selectedValue={selectedPromptId || null}
            options={prompts.map((p) => ({
              value: p.id,
              label: p.name,
            }))}
            onSelect={(value) => handlePromptSelect(value)}
            placeholder={
              prompts.length === 0
                ? t("settings.refine.prompts.noPrompts")
                : t("settings.refine.prompts.selectPrompt")
            }
            disabled={
              isUpdating("post_process_selected_prompt_id") || isCreating
            }
            className="flex-1"
          />
          <Button
            onClick={handleStartCreate}
            variant="default"
            
            disabled={isCreating}
          >
            {t("settings.refine.prompts.createNew")}
          </Button>
        </div>

        {!isCreating && hasPrompts && selectedPrompt && (
          <div className="space-y-3">
            <div className="space-y-2 flex flex-col">
              <label className="text-sm font-semibold">
                {t("settings.refine.prompts.promptLabel")}
              </label>
              <Input
                type="text"
                value={draftName}
                onChange={(e) => setDraftName(e.target.value)}
                placeholder={t(
                  "settings.refine.prompts.promptLabelPlaceholder",
                )}
                
              />
            </div>

            <div className="space-y-2 flex flex-col">
              <label className="text-sm font-semibold">
                {t("settings.refine.prompts.promptInstructions")}
              </label>
              <Textarea
                value={draftText}
                onChange={(e) => setDraftText(e.target.value)}
                placeholder={t(
                  "settings.refine.prompts.promptInstructionsPlaceholder",
                )}
              />
              <p
                className="text-xs text-mid-gray/70"
                dangerouslySetInnerHTML={{
                  __html: t("settings.refine.prompts.promptTip"),
                }}
              />
            </div>

            <div className="flex gap-2 pt-2">
              <Button
                onClick={handleUpdatePrompt}
                variant="default"
                
                disabled={!draftName.trim() || !draftText.trim() || !isDirty}
              >
                {t("settings.refine.prompts.updatePrompt")}
              </Button>
              <Button
                onClick={() => handleDeletePrompt(selectedPromptId)}
                variant="secondary"
                
                disabled={!selectedPromptId || prompts.length <= 1}
              >
                {t("settings.refine.prompts.deletePrompt")}
              </Button>
            </div>
          </div>
        )}

        {!isCreating && !selectedPrompt && (
          <div className="p-3 bg-mid-gray/5 rounded-md border border-mid-gray/20">
            <p className="text-sm text-mid-gray">
              {hasPrompts
                ? t("settings.refine.prompts.selectToEdit")
                : t("settings.refine.prompts.createFirst")}
            </p>
          </div>
        )}

        {isCreating && (
          <div className="space-y-3">
            <div className="space-y-2 block flex flex-col">
              <label className="text-sm font-semibold text-text">
                {t("settings.refine.prompts.promptLabel")}
              </label>
              <Input
                type="text"
                value={draftName}
                onChange={(e) => setDraftName(e.target.value)}
                placeholder={t(
                  "settings.refine.prompts.promptLabelPlaceholder",
                )}
                
              />
            </div>

            <div className="space-y-2 flex flex-col">
              <label className="text-sm font-semibold">
                {t("settings.refine.prompts.promptInstructions")}
              </label>
              <Textarea
                value={draftText}
                onChange={(e) => setDraftText(e.target.value)}
                placeholder={t(
                  "settings.refine.prompts.promptInstructionsPlaceholder",
                )}
              />
              <p
                className="text-xs text-mid-gray/70"
                dangerouslySetInnerHTML={{
                  __html: t("settings.refine.prompts.promptTip"),
                }}
              />
            </div>

            <div className="flex gap-2 pt-2">
              <Button
                onClick={handleCreatePrompt}
                variant="default"
                
                disabled={!draftName.trim() || !draftText.trim()}
              >
                {t("settings.refine.prompts.createPrompt")}
              </Button>
              <Button
                onClick={handleCancelCreate}
                variant="secondary"
                
              >
                {t("settings.refine.prompts.cancel")}
              </Button>
            </div>
          </div>
        )}
      </div>
    </SettingContainer>
  );
};

export const PostProcessingSettingsApi = React.memo(
  PostProcessingSettingsApiComponent,
);
PostProcessingSettingsApi.displayName = "PostProcessingSettingsApi";

export const PostProcessingSettingsPrompts = React.memo(
  PostProcessingSettingsPromptsComponent,
);
PostProcessingSettingsPrompts.displayName = "PostProcessingSettingsPrompts";

interface PostProcessingSettingsProps {
  onNavigate?: (section: string) => void;
}

export const PostProcessingSettings: React.FC<PostProcessingSettingsProps> = ({ onNavigate }) => {
  const { t } = useTranslation();
  const { settings, updateSetting, isUpdating } = useSettings();
  const isEnabled = settings?.post_process_enabled ?? false;
  const autoRefineEnabled = settings?.auto_refine_enabled ?? false;

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      {/* Enable CTA when Refine is disabled */}
      {!isEnabled && (
        <Alert variant="info">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-medium">{t("settings.refine.disabled.title")}</p>
              <p className="text-sm text-muted-foreground">
                {t("settings.refine.disabled.description")}
              </p>
            </div>
            <Button
              onClick={() => updateSetting("post_process_enabled", true)}
              disabled={isUpdating("post_process_enabled")}
              size="sm"
            >
              {t("settings.refine.disabled.enable")}
            </Button>
          </div>
        </Alert>
      )}

      {/* Settings section */}
      <SettingsGroup title={t("settings.refine.behavior.title")}>
        <SettingContainer
          title={t("settings.refine.behavior.autoRefine.title")}
          description={t("settings.refine.behavior.autoRefine.description")}
          descriptionMode="tooltip"
          layout="horizontal"
          grouped={true}
        >
          <Switch
            checked={autoRefineEnabled}
            onCheckedChange={(checked) => updateSetting("auto_refine_enabled", checked)}
            disabled={isUpdating("auto_refine_enabled")}
          />
        </SettingContainer>
      </SettingsGroup>

      <SettingsGroup title={t("settings.refine.api.title")}>
        <PostProcessingSettingsApi onNavigate={onNavigate} />
      </SettingsGroup>

      <SettingsGroup title={t("settings.refine.prompts.title")}>
        <PostProcessingSettingsPrompts />
      </SettingsGroup>
    </div>
  );
};

