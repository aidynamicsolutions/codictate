import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import {
  Music2,
  Instagram,
  Twitter,
  MessageCircle,
  Facebook,
  Linkedin,
  AtSign,
} from "lucide-react";
import { Button } from "@/components/shared/ui/button";
import { Input } from "@/components/shared/ui/input";
import OnboardingLayout from "./OnboardingLayout";
import { MAX_INPUT_LENGTH } from "@/constants";

// Primary referral sources
const PRIMARY_SOURCES = [
  "social_media",
  "youtube",
  "newsletter",
  "ai_chat",
  "search_engine",
  "event",
  "friend",
  "colleague",
  "podcast",
  "article",
  "product_hunt",
  "other",
] as const;

// Secondary options for social media with icons
const SOCIAL_MEDIA_OPTIONS = [
  { id: "tiktok", icon: Music2 },
  { id: "instagram", icon: Instagram },
  { id: "x_twitter", icon: Twitter },
  { id: "discord", icon: MessageCircle },
  { id: "facebook", icon: Facebook },
  { id: "linkedin", icon: Linkedin },
  { id: "reddit", icon: AtSign },
  { id: "threads", icon: AtSign },
  { id: "other", icon: null },
] as const;

type PrimarySource = (typeof PRIMARY_SOURCES)[number];

interface AttributionStepProps {
  userName: string;
  onContinue: (source: string, detail?: string, otherText?: string) => void;
  initialSource?: string;
  initialDetail?: string;
  initialOtherText?: string;
}

export const AttributionStep: React.FC<AttributionStepProps> = ({
  userName,
  onContinue,
  initialSource = "",
  initialDetail = "",
  initialOtherText = "",
}) => {
  const { t } = useTranslation();
  const [selectedSource, setSelectedSource] = useState<string>(initialSource);
  const [selectedDetail, setSelectedDetail] = useState<string>(initialDetail);
  const [otherText, setOtherText] = useState<string>(initialOtherText);

  useEffect(() => {
    setSelectedSource(initialSource);
    setSelectedDetail(initialDetail);
    setOtherText(initialOtherText);
  }, [initialSource, initialDetail, initialOtherText]);

  const selectSource = (source: string) => {
    if (selectedSource === source) {
      // Deselect if already selected
      setSelectedSource("");
      setSelectedDetail("");
      setOtherText("");
    } else {
      setSelectedSource(source);
      setSelectedDetail("");
      setOtherText("");
    }
  };

  const handleOtherTextChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value.slice(0, MAX_INPUT_LENGTH);
    setOtherText(value);
  };

  const selectDetail = (detail: string) => {
    if (selectedDetail === detail) {
      setSelectedDetail("");
    } else {
      setSelectedDetail(detail);
    }
  };

  const handleContinue = () => {
    onContinue(selectedSource, selectedDetail, otherText);
  };

  const displayName = userName || t("onboarding.attribution.defaultName");

  // Check if current source needs secondary options (social_media)
  const showSocialMediaOptions = selectedSource === "social_media";
  const showOtherInput = selectedSource === "other" || selectedDetail === "other";

  // Button styling - unified border-highlight style
  const getButtonClass = (isSelected: boolean) =>
    `rounded-full border-2 px-4 py-2 text-sm font-medium transition-all cursor-pointer ${
      isSelected
        ? "border-primary text-primary bg-primary/5"
        : "border-border bg-background text-foreground hover:border-muted-foreground/50"
    }`;

  return (
    <OnboardingLayout
      currentStep="attribution"
      leftContent={
        <div className="flex flex-col gap-6">
          <div className="flex flex-col gap-2">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
              {t("onboarding.attribution.greeting", { name: displayName })}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.attribution.question")}
            </p>
          </div>

          {/* Primary sources - single choice */}
          <div className="flex flex-wrap gap-2">
            {PRIMARY_SOURCES.map((source) => {
              const isSelected = selectedSource === source;
              return (
                <button
                  key={source}
                  onClick={() => selectSource(source)}
                  className={getButtonClass(isSelected)}
                >
                  {t(`onboarding.attribution.sources.${source}`)}
                </button>
              );
            })}
          </div>

          {/* Secondary options for social media - single choice with icons */}
          {showSocialMediaOptions && (
            <div className="flex flex-col gap-3">
              <p className="text-sm font-medium text-muted-foreground">
                {t("onboarding.attribution.secondaryQuestion.social_media")}
              </p>
              <div className="flex flex-wrap gap-2">
                {SOCIAL_MEDIA_OPTIONS.map(({ id, icon: Icon }) => {
                  const isSelected = selectedDetail === id;
                  return (
                    <button
                      key={id}
                      onClick={() => selectDetail(id)}
                      className={`flex items-center gap-2 ${getButtonClass(isSelected)}`}
                    >
                      {Icon && <Icon className="h-4 w-4" />}
                      {t(`onboarding.attribution.details.social_media.${id}`)}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Other input field */}
          {showOtherInput && (
            <div className="flex flex-col gap-2">
              <p className="text-sm font-medium text-muted-foreground">
                {t("onboarding.attribution.pleaseSpecify")}
              </p>
              <div className="relative max-w-sm">
                <Input
                  type="text"
                  value={otherText}
                  onChange={handleOtherTextChange}
                  placeholder={t("onboarding.attribution.otherPlaceholder")}
                  maxLength={MAX_INPUT_LENGTH}
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                  {otherText.length}/{MAX_INPUT_LENGTH}
                </span>
              </div>
            </div>
          )}

          <Button
            onClick={handleContinue}
            size="lg"
            className="mt-4 w-fit"
            disabled={!selectedSource}
          >
            {t("onboarding.attribution.continue")}
          </Button>
        </div>
      }
      rightContent={
        <img
          src="/src-tauri/resources/svg/undraw_welcome-cats_tw36.svg"
          alt="Welcome illustration"
          className="h-auto max-h-[400px] w-auto max-w-full object-contain"
        />
      }
    />
  );
};

export default AttributionStep;
