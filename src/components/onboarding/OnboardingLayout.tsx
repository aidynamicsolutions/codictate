import React from "react";
import OnboardingProgress, { type OnboardingStep } from "./OnboardingProgress";
import { OnboardingLanguageSwitcher } from "./OnboardingLanguageSwitcher";

interface OnboardingLayoutProps {
  currentStep: OnboardingStep;
  leftContent: React.ReactNode;
  rightContent: React.ReactNode;
}

export const OnboardingLayout: React.FC<OnboardingLayoutProps> = ({
  currentStep,
  leftContent,
  rightContent,
}) => {
  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden">
      {/* Progress indicator at top */}
      <div className="relative shrink-0 border-b border-border bg-background dark:bg-[#1E1E1E]">
        <OnboardingProgress currentStep={currentStep} />
        <div className="absolute right-4 top-1/2 -translate-y-1/2">
          <OnboardingLanguageSwitcher />
        </div>
      </div>

      {/* Split content area */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left side - Form/Content (white/dark background) */}
        <div className="flex w-1/2 flex-col overflow-y-auto bg-background p-8 lg:p-12 dark:bg-[#1E1E1E]">
          <div className="flex flex-1 flex-col justify-center">
            {leftContent}
          </div>
        </div>

        {/* Right side - Illustration (warm background that works with SVG) */}
        <div className="flex w-1/2 items-center justify-center bg-[#FBF5E5] p-8 dark:bg-[#FFFDE8]">
          <div className="flex max-h-full max-w-full items-center justify-center">
            {rightContent}
          </div>
        </div>
      </div>
    </div>
  );
};

export default OnboardingLayout;
