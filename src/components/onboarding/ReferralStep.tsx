import React, { useState, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import OnboardingLayout from "./OnboardingLayout";
import { Link2, Sparkles, Gift } from "lucide-react";

interface ReferralStepProps {
  onComplete: () => void;
  onBack: () => void;
  userName: string;
}

// Hardcoded referral URL for now
const REFERRAL_URL = "https://codictate.com/r/ABCD123";

export const ReferralStep: React.FC<ReferralStepProps> = ({
  onComplete,
  onBack,
  userName,
}) => {
  const { t } = useTranslation();
  const [showToast, setShowToast] = useState(false);
  const [mousePosition, setMousePosition] = useState({ x: 0.5, y: 0.5 });
  const cardRef = useRef<HTMLDivElement>(null);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(REFERRAL_URL);
      setShowToast(true);
      setTimeout(() => setShowToast(false), 3000);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  }, []);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!cardRef.current) return;
    const rect = cardRef.current.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    setMousePosition({ x, y });
  }, []);

  const handleMouseLeave = useCallback(() => {
    setMousePosition({ x: 0.5, y: 0.5 });
  }, []);

  // Calculate 3D transform based on mouse position
  const rotateX = (mousePosition.y - 0.5) * -20; // -10 to 10 degrees
  const rotateY = (mousePosition.x - 0.5) * 20; // -10 to 10 degrees
  const glareX = mousePosition.x * 100;
  const glareY = mousePosition.y * 100;

  const displayName = userName || t("onboarding.attribution.defaultName");

  const howItWorks = [
    {
      icon: Link2,
      text: t("onboarding.referral.steps.share"),
    },
    {
      icon: Sparkles,
      text: t("onboarding.referral.steps.theyGet"),
    },
    {
      icon: Gift,
      text: t("onboarding.referral.steps.youGet"),
    },
  ];

  return (
    <OnboardingLayout
      currentStep="referral"
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
            {/* Title */}
            <div className="flex flex-col gap-2">
              <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
                {t("onboarding.referral.title")}
              </h1>
            </div>

            {/* How it works section */}
            <div className="flex flex-col gap-4">
              <h2 className="text-sm font-medium text-foreground">
                {t("onboarding.referral.howItWorks")}
              </h2>
              <div className="flex flex-col gap-3">
                {howItWorks.map((step, index) => (
                  <div key={index} className="flex items-start gap-3">
                    <step.icon className="h-5 w-5 text-muted-foreground flex-shrink-0 mt-0.5" />
                    <span
                      className="text-sm text-foreground"
                      dangerouslySetInnerHTML={{ __html: step.text }}
                    />
                  </div>
                ))}
              </div>
            </div>

            {/* Referral link section */}
            <div className="flex flex-col gap-2 mt-2">
              <label className="text-sm font-medium text-foreground">
                {t("onboarding.referral.yourLink")}
              </label>
              <div className="flex items-center gap-2">
                <div className="flex items-center gap-2 px-3 py-2 bg-muted rounded-lg flex-1 max-w-[320px]">
                  <Link2 className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                  <span className="text-sm text-foreground truncate font-mono">
                    {REFERRAL_URL}
                  </span>
                </div>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={handleCopy}
                  className="px-4"
                >
                  {t("onboarding.referral.copy")}
                </Button>
              </div>
            </div>
          </div>

          {/* Finish button at bottom */}
          <Button onClick={onComplete} size="lg" className="mt-auto w-fit">
            {t("onboarding.referral.finish")}
          </Button>
        </div>
      }
      rightContent={
        <div className="flex items-center justify-center relative">
          {/* Share Card with 3D effect */}
          {/* Note: CATransformLayer shadow warnings on macOS are safe to ignore - purely informational */}
          <div
            ref={cardRef}
            onMouseMove={handleMouseMove}
            onMouseLeave={handleMouseLeave}
            className="relative cursor-pointer"
            style={{
              perspective: "1000px",
              transformStyle: "preserve-3d",
            }}
          >
            <div
              className="relative w-[380px] h-[240px] rounded-3xl overflow-hidden transition-transform duration-200 ease-out"
              style={{
                transform: `rotateX(${rotateX}deg) rotateY(${rotateY}deg)`,
                transformStyle: "preserve-3d",
                background: "linear-gradient(145deg, #1f5d4d 0%, #14403a 40%, #0a2620 75%, #051512 100%)",
                boxShadow: `
                  0 40px 80px -20px rgba(0, 0, 0, 0.6),
                  0 20px 40px -10px rgba(0, 0, 0, 0.4),
                  0 8px 16px -4px rgba(0, 0, 0, 0.3),
                  0 0 60px -10px rgba(31, 93, 77, 0.3),
                  inset 0 1px 0 rgba(255, 255, 255, 0.15),
                  inset 0 -1px 0 rgba(0, 0, 0, 0.2)
                `,
              }}
            >
              {/* Glare effect */}
              <div
                className="absolute inset-0 pointer-events-none transition-opacity duration-200"
                style={{
                  background: `radial-gradient(circle at ${glareX}% ${glareY}%, rgba(255,255,255,0.2) 0%, transparent 60%)`,
                  opacity: mousePosition.x !== 0.5 || mousePosition.y !== 0.5 ? 1 : 0,
                }}
              />

              {/* Decorative wave pattern - more elegant curves */}
              <div className="absolute top-0 left-0 right-0 h-40 opacity-25">
                <svg viewBox="0 0 380 160" className="w-full h-full" preserveAspectRatio="none">
                  <defs>
                    <linearGradient id="waveGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                      <stop offset="0%" stopColor="rgba(255,255,255,0.15)" />
                      <stop offset="100%" stopColor="rgba(255,255,255,0.05)" />
                    </linearGradient>
                  </defs>
                  <path
                    d="M0 80 Q95 35, 190 80 T380 80 V160 H0 Z"
                    fill="url(#waveGradient)"
                  />
                  <path
                    d="M0 100 Q95 55, 190 100 T380 100 V160 H0 Z"
                    fill="rgba(255,255,255,0.06)"
                  />
                  <path
                    d="M0 120 Q95 85, 190 120 T380 120 V160 H0 Z"
                    fill="rgba(255,255,255,0.03)"
                  />
                </svg>
              </div>

              {/* Subtle shimmer effect */}
              <div 
                className="absolute inset-0 opacity-30 pointer-events-none"
                style={{
                  background: "linear-gradient(105deg, transparent 40%, rgba(255,255,255,0.03) 45%, rgba(255,255,255,0.05) 50%, rgba(255,255,255,0.03) 55%, transparent 60%)",
                }}
              />

              {/* Card content */}
              <div className="absolute inset-0 flex flex-col items-center justify-center p-8">
                {/* Logo and branding */}
                <div className="flex items-center gap-3 mb-3 mt-4">
                  <img
                    src="/src-tauri/icons/icon.png"
                    alt="Codictate"
                    className="h-12 w-12 rounded-xl shadow-lg"
                    style={{ transform: "translateZ(25px)" }}
                  />
                  <span
                    className="text-3xl font-bold text-white tracking-tight"
                    style={{ transform: "translateZ(25px)" }}
                  >
                    {t("appName")}
                  </span>
                  <span
                    className="px-2.5 py-1 rounded-md text-sm font-bold bg-white text-[#0d2b24] shadow-md"
                    style={{ transform: "translateZ(30px)" }}
                  >
                    {t("onboarding.success.proBadge")}
                  </span>
                </div>

                {/* Tagline */}
                <p
                  className="text-xs uppercase tracking-[0.2em] text-white/50 mb-5 font-medium mt-1"
                  style={{ transform: "translateZ(20px)" }}
                >
                  {t("onboarding.referral.cardTagline")}
                </p>

                {/* Gifted by - enhanced pill */}
                <div
                  className="px-4 py-2 rounded-full text-sm font-semibold bg-primary text-primary-foreground shadow-lg mt-10"
                  style={{ 
                    transform: "translateZ(35px)",
                    boxShadow: "0 4px 12px rgba(0,0,0,0.2), 0 0 20px rgba(var(--primary), 0.3)"
                  }}
                >
                  {t("onboarding.referral.giftedBy", { name: displayName })}
                </div>
              </div>
            </div>
          </div>

          {/* Toast notification */}
          <div
            className={`fixed bottom-6 right-6 flex items-center gap-2 px-4 py-3 rounded-lg bg-foreground text-background shadow-lg transition-all duration-300 ${
              showToast
                ? "opacity-100 translate-y-0"
                : "opacity-0 translate-y-4 pointer-events-none"
            }`}
          >
            <svg
              className="h-4 w-4 text-green-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M5 13l4 4L19 7"
              />
            </svg>
            <span className="text-sm font-medium">
              {t("onboarding.referral.copied")}
            </span>
          </div>
        </div>
      }
    />
  );
};

export default ReferralStep;
