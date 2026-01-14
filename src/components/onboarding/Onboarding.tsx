import React, { useState, useEffect } from "react";
import { useSettings } from "@/hooks/useSettings";
import WelcomeStep from "./WelcomeStep";
import AttributionStep from "./AttributionStep";
import TellUsAboutYouStep from "./TellUsAboutYouStep";
import PermissionsStep from "./PermissionsStep";
import SetupStep from "./SetupStep";
import LearnStep from "./LearnStep";
import type { OnboardingStep } from "./OnboardingProgress";

interface OnboardingProps {
  onComplete: () => void;
}

const STEP_ORDER: OnboardingStep[] = [
  "welcome",
  "attribution",
  "tellUsAboutYou",
  "permissions",
  "setup",
  "learn",
];

const Onboarding: React.FC<OnboardingProps> = ({ onComplete }) => {
  const { settings, updateSetting } = useSettings();
  const [currentStep, setCurrentStep] = useState<OnboardingStep>("welcome");
  const [userName, setUserName] = useState<string>("");
  // Attribution state
  const [referralSource, setReferralSource] = useState<string>("");
  const [referralDetail, setReferralDetail] = useState<string>("");
  const [referralOtherText, setReferralOtherText] = useState<string>("");
  // Work profile state
  const [workRole, setWorkRole] = useState<string>("");
  const [professionalLevel, setProfessionalLevel] = useState<string>("");
  const [workRoleOther, setWorkRoleOther] = useState<string>("");

  // Initialize state from settings on mount
  useEffect(() => {
    if (settings) {
      // Resume from saved step
      const savedStep = settings.onboarding_step ?? 0;
      if (savedStep > 0 && savedStep <= STEP_ORDER.length) {
        setCurrentStep(STEP_ORDER[savedStep - 1] || "welcome");
      }

      // Restore saved data
      if (settings.user_name) {
        setUserName(settings.user_name);
      }
      // Handle both old array format and new single-choice format
      if (settings.referral_sources && settings.referral_sources.length > 0) {
        setReferralSource(settings.referral_sources[0]);
      }
      if (settings.referral_details) {
        const details = settings.referral_details as Record<string, string[]>;
        const firstSource = Object.keys(details)[0];
        if (firstSource && details[firstSource]?.length > 0) {
          setReferralDetail(details[firstSource][0]);
        }
      }
      // Work profile
      if (settings.work_role) {
        setWorkRole(settings.work_role);
      }
      if (settings.professional_level) {
        setProfessionalLevel(settings.professional_level);
      }
    }
  }, [settings]);

  const saveProgress = async (step: OnboardingStep) => {
    const stepIndex = STEP_ORDER.indexOf(step) + 1;
    await updateSetting("onboarding_step", stepIndex);
  };

  const goToNextStep = async () => {
    const currentIndex = STEP_ORDER.indexOf(currentStep);
    if (currentIndex < STEP_ORDER.length - 1) {
      const nextStep = STEP_ORDER[currentIndex + 1];
      setCurrentStep(nextStep);
      await saveProgress(nextStep);
    }
  };

  const handleWelcomeContinue = async (name: string) => {
    setUserName(name);
    await updateSetting("user_name", name || null);
    await goToNextStep();
  };

  const handleAboutYouContinue = async (
    source: string,
    detail?: string,
    otherText?: string
  ) => {
    setReferralSource(source);
    setReferralDetail(detail || "");
    setReferralOtherText(otherText || "");

    // Store as arrays for backward compatibility with settings schema
    await updateSetting("referral_sources", source ? [source] : []);
    if (detail) {
      await updateSetting("referral_details", { [source]: [detail] });
    } else if (otherText) {
      await updateSetting("referral_details", { [source]: [otherText] });
    } else {
      await updateSetting("referral_details", {});
    }
    await goToNextStep();
  };

  const handleTellUsAboutYouContinue = async (
    role: string,
    level?: string,
    otherText?: string
  ) => {
    setWorkRole(role);
    setProfessionalLevel(level || "");
    setWorkRoleOther(otherText || "");

    await updateSetting("work_role", role || null);
    await updateSetting("professional_level", level || null);
    await updateSetting("work_role_other", otherText || null);
    await goToNextStep();
  };

  const handlePermissionsContinue = async () => {
    await goToNextStep();
  };

  const handleSetupContinue = async () => {
    await goToNextStep();
  };

  const handleLearnComplete = async () => {
    await updateSetting("onboarding_completed", true);
    await updateSetting("onboarding_step", STEP_ORDER.length + 1);
    onComplete();
  };

  // Render current step
  switch (currentStep) {
    case "welcome":
      return (
        <WelcomeStep onContinue={handleWelcomeContinue} initialName={userName} />
      );
    case "attribution":
      return (
        <AttributionStep
          userName={userName}
          onContinue={handleAboutYouContinue}
          initialSource={referralSource}
          initialDetail={referralDetail}
          initialOtherText={referralOtherText}
        />
      );
    case "tellUsAboutYou":
      return (
        <TellUsAboutYouStep
          onContinue={handleTellUsAboutYouContinue}
          initialWorkRole={workRole}
          initialProfessionalLevel={professionalLevel}
        />
      );
    case "permissions":
      return <PermissionsStep onContinue={handlePermissionsContinue} />;
    case "setup":
      return <SetupStep onContinue={handleSetupContinue} />;
    case "learn":
      return <LearnStep onComplete={handleLearnComplete} />;
    default:
      return (
        <WelcomeStep onContinue={handleWelcomeContinue} initialName={userName} />
      );
  }
};

export default Onboarding;
