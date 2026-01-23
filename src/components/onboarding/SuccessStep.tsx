import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import confetti from "canvas-confetti";
import { Button } from "@/components/shared/ui/button";
import OnboardingLayout from "./OnboardingLayout";
import { Infinity, Globe, Sparkles, Terminal } from "lucide-react";

interface SuccessStepProps {
  onComplete: () => void;
  onBack: () => void;
}

export const SuccessStep: React.FC<SuccessStepProps> = ({
  onComplete,
  onBack,
}) => {
  const { t } = useTranslation();

  // Fire confetti explosion on mount
  useEffect(() => {
    const colors = ["#FFD700", "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7", "#DDA0DD", "#98D8C8", "#FF69B4", "#00CED1"];

    // Function to fire a burst
    const fireBurst = (originX: number, originY: number, particleCount: number, spread: number, startVelocity: number) => {
      confetti({
        particleCount,
        spread,
        startVelocity,
        origin: { x: originX, y: originY },
        colors,
        ticks: 200,
        gravity: 0.8,
        scalar: 1.2,
        drift: 0,
      });
    };

    // Initial big explosion from center
    fireBurst(0.5, 0.5, 100, 360, 45);

    // Follow-up bursts with slight delay
    setTimeout(() => {
      fireBurst(0.3, 0.5, 50, 120, 35);
      fireBurst(0.7, 0.5, 50, 120, 35);
    }, 150);

    setTimeout(() => {
      fireBurst(0.5, 0.4, 60, 180, 40);
    }, 300);

    setTimeout(() => {
      fireBurst(0.4, 0.6, 40, 100, 30);
      fireBurst(0.6, 0.6, 40, 100, 30);
    }, 450);

    // Final celebratory burst
    setTimeout(() => {
      fireBurst(0.5, 0.5, 80, 360, 50);
    }, 600);

  }, []);

  const features = [
    {
      icon: Infinity,
      label: t("onboarding.success.features.unlimited"),
    },
    {
      icon: Globe,
      label: t("onboarding.success.features.collaboration"),
    },
    {
      icon: Sparkles,
      label: t("onboarding.success.features.crossPlatform"),
    },
    {
      icon: Terminal,
      label: t("onboarding.success.features.commandMode"),
    },
  ];

  return (
    <OnboardingLayout
      currentStep="learn"
      leftContent={
        <div className="flex flex-col h-full">
          {/* Back button */}
          <button
            onClick={onBack}
            className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors mb-6 w-fit"
          >
            ← {t("onboarding.learn.back")}
          </button>

          {/* Content centered vertically */}
          <div className="flex flex-col gap-6 my-auto">
            {/* Badge */}
            <span className="inline-flex items-center px-3 py-1 rounded-full text-xs font-semibold uppercase tracking-wide bg-foreground text-background w-fit">
              {t("onboarding.success.badge")}
            </span>

            {/* Title and description */}
            <div className="flex flex-col gap-2">
              <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
                {t("onboarding.success.title")}
              </h1>
              <p className="text-muted-foreground">
                {t("onboarding.success.description")}
              </p>
              <p className="text-muted-foreground text-sm">
                {t("onboarding.success.noCard")}
              </p>
            </div>

            {/* Features list */}
            <div className="flex flex-col gap-3 mt-4">
              {features.map((feature, index) => (
                <div key={index} className="flex items-center gap-3">
                  <feature.icon className="h-5 w-5 text-muted-foreground" />
                  <span className="text-sm text-foreground">
                    {feature.label}
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* Continue button at bottom */}
          <Button onClick={onComplete} size="lg" className="mt-auto w-fit">
            {t("onboarding.success.continue")}
          </Button>
        </div>
      }
      rightContent={
        <div className="flex items-center justify-center gap-4">
          {/* App icon */}
          <img
            src="/src-tauri/icons/icon.png"
            alt="Codictate"
            className="h-20 w-20 rounded-2xl shadow-lg"
          />
          {/* App name with Pro badge - inline */}
          <div className="flex items-center gap-2">
            <span className="text-4xl font-bold text-black">
              {t("appName")}
            </span>
            <span className="px-2 py-1 rounded-md text-sm font-semibold bg-primary text-primary-foreground">
              {t("onboarding.success.proBadge")}
            </span>
          </div>
        </div>
      }
    />
  );
};

export default SuccessStep;

