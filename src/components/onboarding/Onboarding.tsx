import React, { useState, useEffect } from "react";
import { useUserProfile } from "@/hooks/useUserProfile";
import WelcomeStep from "./WelcomeStep";
import AttributionStep from "./AttributionStep";
import TellUsAboutYouStep from "./TellUsAboutYouStep";
import TypingUseCasesStep from "./TypingUseCasesStep";
import PermissionsStep from "./PermissionsStep";
import MicrophoneCheckStep from "./MicrophoneCheckStep";
import HotkeySetupStep from "./HotkeySetupStep";
import LanguageSelectStep from "./LanguageSelectStep";
import LearnStep from "./LearnStep";
import type { OnboardingStep } from "./OnboardingProgress";

interface OnboardingProps {
  onComplete: () => void;
}

const STEP_ORDER: OnboardingStep[] = [
  "welcome",
  "attribution",
  "tellUsAboutYou",
  "typingUseCases",
  "permissions",
  "microphoneCheck",
  "hotkeySetup",
  "languageSelect",
  "learn",
];

const Onboarding: React.FC<OnboardingProps> = ({ onComplete }) => {
  const { profile, updateProfile } = useUserProfile();
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
  // Typing use cases state
  const [typingUseCases, setTypingUseCases] = useState<string[]>([]);
  const [typingUseCasesOther, setTypingUseCasesOther] = useState<string>("");

  // Initialize state from profile on mount
  useEffect(() => {
    if (profile) {
      // Resume from saved step
      const savedStep = profile.onboarding_step ?? 0;
      if (savedStep > 0 && savedStep <= STEP_ORDER.length) {
        setCurrentStep(STEP_ORDER[savedStep - 1] || "welcome");
      }

      // Restore saved data
      if (profile.user_name) {
        setUserName(profile.user_name);
      }
      // Handle both old array format and new single-choice format
      if (profile.referral_sources && profile.referral_sources.length > 0) {
        setReferralSource(profile.referral_sources[0]);
      }
      if (profile.referral_details) {
        const details = profile.referral_details as Record<string, string[]>;
        const firstSource = Object.keys(details)[0];
        if (firstSource && details[firstSource]?.length > 0) {
          setReferralDetail(details[firstSource][0]);
        }
      }
      // Work profile
      if (profile.work_role) {
        setWorkRole(profile.work_role);
      }
      if (profile.professional_level) {
        setProfessionalLevel(profile.professional_level);
      }
      // Typing use cases
      if (profile.typing_use_cases && profile.typing_use_cases.length > 0) {
        setTypingUseCases(profile.typing_use_cases);
      }
      if (profile.typing_use_cases_other) {
        setTypingUseCasesOther(profile.typing_use_cases_other);
      }
    }
  }, [profile]);

  const saveProgress = async (step: OnboardingStep) => {
    const stepIndex = STEP_ORDER.indexOf(step) + 1;
    await updateProfile("onboarding_step", stepIndex);
  };

  const goToNextStep = async () => {
    const currentIndex = STEP_ORDER.indexOf(currentStep);
    if (currentIndex < STEP_ORDER.length - 1) {
      const nextStep = STEP_ORDER[currentIndex + 1];
      setCurrentStep(nextStep);
      await saveProgress(nextStep);
    }
  };

  const goToPreviousStep = async () => {
    const currentIndex = STEP_ORDER.indexOf(currentStep);
    if (currentIndex > 0) {
      const prevStep = STEP_ORDER[currentIndex - 1];
      setCurrentStep(prevStep);
      await saveProgress(prevStep);
    }
  };

  const handleWelcomeContinue = async (name: string) => {
    setUserName(name);
    await updateProfile("user_name", name || null);
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

    // Store as arrays for backward compatibility with profile schema
    await updateProfile("referral_sources", source ? [source] : []);
    if (detail) {
      await updateProfile("referral_details", { [source]: [detail] });
    } else if (otherText) {
      await updateProfile("referral_details", { [source]: [otherText] });
    } else {
      await updateProfile("referral_details", {});
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

    await updateProfile("work_role", role || null);
    await updateProfile("professional_level", level || null);
    await updateProfile("work_role_other", otherText || null);
    await goToNextStep();
  };

  const handleTypingUseCasesContinue = async (
    useCases: string[],
    otherText?: string
  ) => {
    setTypingUseCases(useCases);
    setTypingUseCasesOther(otherText || "");

    await updateProfile("typing_use_cases", useCases);
    await updateProfile("typing_use_cases_other", otherText || null);
    await goToNextStep();
  };

  const handlePermissionsContinue = async () => {
    await goToNextStep();
  };

  const handlePermissionsBack = async () => {
    await goToPreviousStep();
  };

  const handleMicrophoneCheckContinue = async () => {
    await goToNextStep();
  };

  const handleMicrophoneCheckBack = async () => {
    await goToPreviousStep();
  };

  const handleHotkeySetupContinue = async () => {
    await goToNextStep();
  };

  const handleHotkeySetupBack = async () => {
    await goToPreviousStep();
  };

  const handleLanguageSelectContinue = async () => {
    await goToNextStep();
  };

  const handleLanguageSelectBack = async () => {
    await goToPreviousStep();
  };

  const handleLearnComplete = async () => {
    await updateProfile("onboarding_completed", true);
    await updateProfile("onboarding_step", STEP_ORDER.length + 1);
    onComplete();
  };

  const handleLearnBack = async () => {
    await goToPreviousStep();
  };

  const handleAttributionBack = async () => {
    await goToPreviousStep();
  };

  const handleTellUsAboutYouBack = async () => {
    await goToPreviousStep();
  };

  const handleTypingUseCasesBack = async () => {
    await goToPreviousStep();
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
          onBack={handleAttributionBack}
          initialSource={referralSource}
          initialDetail={referralDetail}
          initialOtherText={referralOtherText}
        />
      );
    case "tellUsAboutYou":
      return (
        <TellUsAboutYouStep
          onContinue={handleTellUsAboutYouContinue}
          onBack={handleTellUsAboutYouBack}
          initialWorkRole={workRole}
          initialProfessionalLevel={professionalLevel}
        />
      );
    case "typingUseCases":
      return (
        <TypingUseCasesStep
          onContinue={handleTypingUseCasesContinue}
          onBack={handleTypingUseCasesBack}
          initialUseCases={typingUseCases}
          initialOtherText={typingUseCasesOther}
        />
      );
    case "permissions":
      return <PermissionsStep onContinue={handlePermissionsContinue} onBack={handlePermissionsBack} />;
    case "microphoneCheck":
      return (
        <MicrophoneCheckStep
          onContinue={handleMicrophoneCheckContinue}
          onBack={handleMicrophoneCheckBack}
        />
      );
    case "hotkeySetup":
      return (
        <HotkeySetupStep
          onContinue={handleHotkeySetupContinue}
          onBack={handleHotkeySetupBack}
        />
      );
    case "languageSelect":
      return (
        <LanguageSelectStep
          onContinue={handleLanguageSelectContinue}
          onBack={handleLanguageSelectBack}
        />
      );
    case "learn":
      return (
        <LearnStep
          onComplete={handleLearnComplete}
          onBack={handleLearnBack}
          onSkip={handleLearnComplete}
          userName={userName}
        />
      );
    default:
      return (
        <WelcomeStep onContinue={handleWelcomeContinue} initialName={userName} />
      );
  }
};

export default Onboarding;
