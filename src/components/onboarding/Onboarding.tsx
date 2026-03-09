import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { commands } from "@/bindings";
import { useModelStore } from "@/stores/modelStore";
import { useUserProfile } from "@/hooks/useUserProfile";
import WelcomeStep from "./WelcomeStep";
import AttributionStep from "./AttributionStep";
import TellUsAboutYouStep from "./TellUsAboutYouStep";
import TypingUseCasesStep from "./TypingUseCasesStep";
import PermissionsStep from "./PermissionsStep";
import ModelDownloadStep from "./ModelDownloadStep";
import ModelDownloadProgress from "./ModelDownloadProgress";
import MicrophoneCheckStep from "./MicrophoneCheckStep";
import HotkeySetupStep from "./HotkeySetupStep";
import LanguageSelectStep from "./LanguageSelectStep";
import LearnStep from "./LearnStep";
import SuccessStep from "./SuccessStep";
import ReferralStep from "./ReferralStep";
import {
  ALL_ONBOARDING_STEPS,
  type OnboardingStep,
} from "./flow";
import { recordOnboardingActivationAttempt } from "./onboardingRuntime";
import { logError } from "@/utils/logging";

interface OnboardingProps {
  onComplete: () => void;
}

interface ModelStateEvent {
  event_type: string;
  model_id?: string;
  error?: string;
}

const Onboarding: React.FC<OnboardingProps> = ({ onComplete }) => {
  const { profile, isLoading, refreshProfile, updateProfile } = useUserProfile();
  const [currentStep, setCurrentStep] = useState<OnboardingStep>("welcome");
  const modelStore = useModelStore();
  const selectModel = modelStore.selectModel;
  const currentModelId = modelStore.currentModel;
  const [onboardingTargetModelId, setOnboardingTargetModelId] = useState("");
  const trackedModelId = onboardingTargetModelId || currentModelId;
  const trackedModelInfo = trackedModelId
    ? modelStore.getModelInfo(trackedModelId)
    : undefined;
  const isTrackedModelDownloaded = trackedModelInfo?.is_downloaded ?? false;
  const isTrackedModelDownloading = trackedModelId
    ? modelStore.isModelDownloading(trackedModelId)
    : false;
  const isTrackedModelExtracting = trackedModelId
    ? modelStore.isModelExtracting(trackedModelId)
    : false;
  const isTrackedModelSelected =
    Boolean(trackedModelId) && currentModelId === trackedModelId;
  const [backendModelLoaded, setBackendModelLoaded] = useState(false);
  const [backendModelLoading, setBackendModelLoading] = useState(false);
  const [backendModelReady, setBackendModelReady] = useState(false);
  const [backendModelPreparing, setBackendModelPreparing] = useState(false);
  const [isSelectingTargetModel, setIsSelectingTargetModel] = useState(false);

  // Learn-step activation should only proceed once the selected model is ready.
  const modelReady = useMemo(() => {
    if (!trackedModelId) return false;
    if (!isTrackedModelDownloaded) return false;
    if (isTrackedModelDownloading) return false;
    if (isTrackedModelExtracting) return false;
    if (!isTrackedModelSelected) return false;
    return backendModelLoaded && backendModelReady;
  }, [
    backendModelLoaded,
    backendModelReady,
    isTrackedModelDownloaded,
    isTrackedModelDownloading,
    isTrackedModelExtracting,
    isTrackedModelSelected,
    trackedModelId,
  ]);
  const [userName, setUserName] = useState("");
  const [learnActivated, setLearnActivated] = useState(false);
  const [referralSource, setReferralSource] = useState("");
  const [referralDetail, setReferralDetail] = useState("");
  const [referralOtherText, setReferralOtherText] = useState("");
  const [workRole, setWorkRole] = useState("");
  const [professionalLevel, setProfessionalLevel] = useState("");
  const [workRoleOther, setWorkRoleOther] = useState("");
  const [typingUseCases, setTypingUseCases] = useState<string[]>([]);
  const [typingUseCasesOther, setTypingUseCasesOther] = useState("");
  const activationRecordedRef = useRef(false);
  const preloadAttemptRef = useRef<{
    modelId: string;
    step: OnboardingStep;
  } | null>(null);
  // After the first hydration, step navigation is component-owned.
  // Backend profile refreshes (e.g. user-profile-updated events) must NOT
  // rewrite currentStep — doing so would rewind the user mid-flow.
  const hydratedRef = useRef(false);

  const handleOnboardingActionError = (action: string, error: unknown) => {
    logError(
      `event=onboarding_action_failed action=${action} error=${error}`,
      "fe-onboarding",
    );
    void refreshProfile();
  };

  const runOnboardingAction = async (
    action: string,
    callback: () => Promise<void>,
  ) => {
    try {
      await callback();
    } catch (error) {
      handleOnboardingActionError(action, error);
    }
  };

  useEffect(() => {
    if (!trackedModelId) {
      setBackendModelLoaded(false);
      setBackendModelLoading(false);
      setBackendModelReady(false);
      setBackendModelPreparing(false);
      return;
    }

    let ignore = false;
    let unlisten: (() => void) | undefined;

    const syncModelLoadStatus = async () => {
      try {
        const result = await commands.getModelLoadStatus();
        if (ignore || result.status !== "ok") {
          return;
        }

        const isTrackedModel = result.data.current_model === trackedModelId;
        const isTrackedModelLoaded = result.data.is_loaded && isTrackedModel;
        const isTrackedModelReady = result.data.is_warmed && isTrackedModel;

        setBackendModelLoaded(isTrackedModelLoaded);
        setBackendModelLoading(result.data.is_loading && !isTrackedModelLoaded);
        setBackendModelReady(isTrackedModelReady);
        setBackendModelPreparing(result.data.is_warming && !isTrackedModelReady);
      } catch (error) {
        if (!ignore) {
          logError(
            `event=onboarding_model_load_status_check_failed model=${trackedModelId} error=${error}`,
            "fe-onboarding",
          );
        }
      }
    };

    const setup = async () => {
      await syncModelLoadStatus();
      unlisten = await listen<ModelStateEvent>("model-state-changed", (event) => {
        if (ignore) {
          return;
        }

        const { event_type, model_id } = event.payload;
        if (model_id && model_id !== trackedModelId) {
          return;
        }

        switch (event_type) {
          case "loading_started":
            setBackendModelLoaded(false);
            setBackendModelLoading(true);
            setBackendModelReady(false);
            setBackendModelPreparing(false);
            break;
          case "loading_completed":
            setBackendModelLoaded(true);
            setBackendModelLoading(false);
            setBackendModelReady(true);
            setBackendModelPreparing(false);
            break;
          case "loading_failed":
          case "unloaded":
            setBackendModelLoaded(false);
            setBackendModelLoading(false);
            setBackendModelReady(false);
            setBackendModelPreparing(false);
            break;
        }
      });
    };

    void setup();

    return () => {
      ignore = true;
      unlisten?.();
    };
  }, [trackedModelId]);

  useEffect(() => {
    if (
      !onboardingTargetModelId ||
      !isTrackedModelDownloaded ||
      isTrackedModelDownloading ||
      isTrackedModelExtracting ||
      isTrackedModelSelected ||
      isSelectingTargetModel
    ) {
      return;
    }

    let ignore = false;
    setIsSelectingTargetModel(true);
    void selectModel(onboardingTargetModelId)
      .then((ok) => {
        if (!ok && !ignore) {
          logError(
            `event=onboarding_target_model_select_failed model=${onboardingTargetModelId}`,
            "fe-onboarding",
          );
        }
      })
      .catch((error) => {
        if (!ignore) {
          logError(
            `event=onboarding_target_model_select_failed model=${onboardingTargetModelId} error=${error}`,
            "fe-onboarding",
          );
        }
      })
      .finally(() => {
        if (!ignore) {
          setIsSelectingTargetModel(false);
        }
      });

    return () => {
      ignore = true;
    };
  }, [
    isSelectingTargetModel,
    isTrackedModelDownloaded,
    isTrackedModelDownloading,
    isTrackedModelExtracting,
    isTrackedModelSelected,
    onboardingTargetModelId,
    selectModel,
  ]);

  useEffect(() => {
    const isEligiblePreloadStep =
      currentStep === "microphoneCheck" ||
      currentStep === "hotkeySetup" ||
      currentStep === "languageSelect" ||
      currentStep === "learn";

    if (
      !isEligiblePreloadStep ||
      !trackedModelId ||
      !isTrackedModelDownloaded ||
      isTrackedModelDownloading ||
      isTrackedModelExtracting ||
      !isTrackedModelSelected ||
      backendModelReady ||
      backendModelPreparing
    ) {
      return;
    }

    const hasAttemptedPreloadForCurrentStep =
      preloadAttemptRef.current?.modelId === trackedModelId &&
      preloadAttemptRef.current?.step === currentStep;
    if (hasAttemptedPreloadForCurrentStep) {
      return;
    }

    preloadAttemptRef.current = {
      modelId: trackedModelId,
      step: currentStep,
    };
    void commands.warmUpTranscriptionModel(trackedModelId).catch((error) => {
      // Allow retry on the next state transition if this preload attempt fails.
      preloadAttemptRef.current = null;
      logError(
        `event=onboarding_model_preload_failed step=${currentStep} model=${trackedModelId} error=${error}`,
        "fe-onboarding",
      );
    });
  }, [
    backendModelLoaded,
    backendModelLoading,
    backendModelReady,
    backendModelPreparing,
    currentStep,
    isTrackedModelDownloaded,
    isTrackedModelDownloading,
    isTrackedModelExtracting,
    isTrackedModelSelected,
    trackedModelId,
  ]);

  useEffect(() => {
    if (isLoading) {
      return;
    }

    // Derive step position from the persisted profile only on first hydration.
    // After that, step transitions are local — stale profile refreshes must
    // never overwrite in-progress navigation.
    if (profile && !hydratedRef.current) {
      hydratedRef.current = true;

      const savedStep = profile.onboarding_step ?? 0;
      if (
        profile.onboarding_home_guidance_active &&
        !profile.onboarding_completed
      ) {
        setCurrentStep("success");
      } else if (savedStep > 0 && savedStep <= ALL_ONBOARDING_STEPS.length) {
        setCurrentStep(ALL_ONBOARDING_STEPS[savedStep - 1] || "welcome");
      }

      activationRecordedRef.current =
        profile.onboarding_activation_completed ?? false;
      setLearnActivated(profile.onboarding_activation_completed ?? false);
    }

    // Non-navigation fields are safe to sync on every profile change.
    if (profile) {
      if (profile.user_name) {
        setUserName(profile.user_name);
      }
      if (profile.referral_sources?.length) {
        setReferralSource(profile.referral_sources[0]);
      }
      if (profile.referral_details) {
        const details = profile.referral_details as Record<string, string[]>;
        const firstSource = Object.keys(details)[0];
        if (firstSource && details[firstSource]?.length > 0) {
          setReferralDetail(details[firstSource][0]);
        }
      }
      if (profile.work_role) {
        setWorkRole(profile.work_role);
      }
      if (profile.professional_level) {
        setProfessionalLevel(profile.professional_level);
      }
      if (profile.work_role_other) {
        setWorkRoleOther(profile.work_role_other);
      }
      if (profile.typing_use_cases?.length) {
        setTypingUseCases(profile.typing_use_cases);
      }
      if (profile.typing_use_cases_other) {
        setTypingUseCasesOther(profile.typing_use_cases_other);
      }
    }

  }, [isLoading, profile]);

  useLayoutEffect(() => {
    if (isLoading || profile?.onboarding_started) {
      return;
    }

    void commands
      .markOnboardingStartedCommand()
      .then((result) => {
        if (result.status === "error") {
          logError(
            `event=onboarding_start_mark_failed error=${result.error}`,
            "fe-onboarding",
          );
          return;
        }

        if (result.data) {
          void refreshProfile();
        }
      })
      .catch((error) => {
        logError(
          `event=onboarding_start_mark_failed error=${error}`,
          "fe-onboarding",
        );
      });
  }, [isLoading, profile?.onboarding_started, refreshProfile]);

  const saveProgress = async (step: OnboardingStep) => {
    const stepIndex = ALL_ONBOARDING_STEPS.indexOf(step) + 1;
    await updateProfile("onboarding_step", stepIndex);
  };

  const completeStepAndGoToNextStep = async (step: OnboardingStep) => {
    const currentIndex = ALL_ONBOARDING_STEPS.indexOf(step);
    if (currentIndex < ALL_ONBOARDING_STEPS.length - 1) {
      const nextStep = ALL_ONBOARDING_STEPS[currentIndex + 1];
      await saveProgress(nextStep);
      setCurrentStep(nextStep);
    }
  };

  const goToPreviousStep = async () => {
    try {
      const currentIndex = ALL_ONBOARDING_STEPS.indexOf(currentStep);
      if (currentIndex > 0) {
        const previousStep = ALL_ONBOARDING_STEPS[currentIndex - 1];
        await saveProgress(previousStep);
        setCurrentStep(previousStep);
      }
    } catch (error) {
      handleOnboardingActionError("go_to_previous_step", error);
    }
  };

  const recordOnboardingActivation = async (
    surface: "learn_mock_chat" | "post_onboarding",
  ) => {
    await recordOnboardingActivationAttempt({
      activationRecorded: activationRecordedRef.current,
      surface,
      recordActivation: (activationSurface) =>
        commands.recordOnboardingActivationCommand(activationSurface),
      setLearnActivated,
      markActivationRecorded: () => {
        activationRecordedRef.current = true;
      },
      trackLearnCompleted: async () => {},
      logError: (message) => {
        logError(message, "fe-onboarding");
      },
    });
  };

  const handleWelcomeContinue = async (name: string) => {
    await runOnboardingAction("welcome_continue", async () => {
      setUserName(name);
      await updateProfile("user_name", name || null);
      await completeStepAndGoToNextStep("welcome");
    });
  };

  const handleAboutYouContinue = async (
    source: string,
    detail?: string,
    otherText?: string,
  ) => {
    await runOnboardingAction("attribution_continue", async () => {
      setReferralSource(source);
      setReferralDetail(detail || "");
      setReferralOtherText(otherText || "");

      await updateProfile("referral_sources", source ? [source] : []);
      if (detail) {
        await updateProfile("referral_details", { [source]: [detail] });
      } else if (otherText) {
        await updateProfile("referral_details", { [source]: [otherText] });
      } else {
        await updateProfile("referral_details", {});
      }

      await completeStepAndGoToNextStep("attribution");
    });
  };

  const handleTellUsAboutYouContinue = async (
    role: string,
    level?: string,
    otherText?: string,
  ) => {
    await runOnboardingAction("tell_us_about_you_continue", async () => {
      setWorkRole(role);
      setProfessionalLevel(level || "");
      setWorkRoleOther(otherText || "");

      await updateProfile("work_role", role || null);
      await updateProfile("professional_level", level || null);
      await updateProfile("work_role_other", otherText || null);

      await completeStepAndGoToNextStep("tellUsAboutYou");
    });
  };

  const handleTypingUseCasesContinue = async (
    useCases: string[],
    otherText?: string,
  ) => {
    await runOnboardingAction("typing_use_cases_continue", async () => {
      setTypingUseCases(useCases);
      setTypingUseCasesOther(otherText || "");

      await updateProfile("typing_use_cases", useCases);
      await updateProfile("typing_use_cases_other", otherText || null);

      await completeStepAndGoToNextStep("typingUseCases");
    });
  };

  const handlePermissionsContinue = async () => {
    await runOnboardingAction("permissions_continue", async () => {
      await completeStepAndGoToNextStep("permissions");
    });
  };

  const handleDownloadModelContinue = async () => {
    await runOnboardingAction("download_model_continue", async () => {
      await completeStepAndGoToNextStep("downloadModel");
    });
  };

  const handleRecommendedModelResolved = useCallback((modelId: string) => {
    setOnboardingTargetModelId((previous) =>
      previous === modelId ? previous : modelId,
    );
  }, []);

  const handleMicrophoneCheckContinue = async () => {
    await runOnboardingAction("microphone_check_continue", async () => {
      await completeStepAndGoToNextStep("microphoneCheck");
    });
  };

  const handleHotkeySetupContinue = async () => {
    await runOnboardingAction("hotkey_setup_continue", async () => {
      await completeStepAndGoToNextStep("hotkeySetup");
    });
  };

  const handleLanguageSelectContinue = async () => {
    await runOnboardingAction("language_select_continue", async () => {
      await completeStepAndGoToNextStep("languageSelect");
    });
  };

  const handleLearnContinue = async () => {
    await runOnboardingAction("learn_continue", async () => {
      await saveProgress("success");
      setCurrentStep("success");
    });
  };

  const handleLearnSkip = async () => {
    await runOnboardingAction("learn_skip", async () => {
      await saveProgress("success");
      setCurrentStep("success");
    });
  };

  const handleSuccessComplete = async () => {
    await runOnboardingAction("success_continue", async () => {
      await completeStepAndGoToNextStep("success");
    });
  };

  const handleReferralComplete = async () => {
    await runOnboardingAction("referral_complete", async () => {
      await updateProfile("onboarding_completed", true);
      await updateProfile("onboarding_home_guidance_active", false);
      await updateProfile("onboarding_step", ALL_ONBOARDING_STEPS.length + 1);
      onComplete();
    });
  };

  const renderStep = () => {
    switch (currentStep) {
      case "welcome":
        return (
          <WelcomeStep
            onContinue={handleWelcomeContinue}
            initialName={userName}
          />
        );
      case "attribution":
        return (
          <AttributionStep
            userName={userName}
            onContinue={handleAboutYouContinue}
            onBack={goToPreviousStep}
            initialSource={referralSource}
            initialDetail={referralDetail}
            initialOtherText={referralOtherText}
          />
        );
      case "tellUsAboutYou":
        return (
          <TellUsAboutYouStep
            onContinue={handleTellUsAboutYouContinue}
            onBack={goToPreviousStep}
            initialWorkRole={workRole}
            initialProfessionalLevel={professionalLevel}
          />
        );
      case "typingUseCases":
        return (
          <TypingUseCasesStep
            onContinue={handleTypingUseCasesContinue}
            onBack={goToPreviousStep}
            initialUseCases={typingUseCases}
            initialOtherText={typingUseCasesOther}
          />
        );
      case "permissions":
        return (
          <PermissionsStep
            onContinue={handlePermissionsContinue}
            onBack={goToPreviousStep}
          />
        );
      case "downloadModel":
        return (
          <ModelDownloadStep
            onContinue={handleDownloadModelContinue}
            onBack={goToPreviousStep}
            onRecommendedModelResolved={handleRecommendedModelResolved}
          />
        );
      case "microphoneCheck":
        return (
          <MicrophoneCheckStep
            onContinue={handleMicrophoneCheckContinue}
            onBack={goToPreviousStep}
          />
        );
      case "hotkeySetup":
        return (
          <HotkeySetupStep
            onContinue={handleHotkeySetupContinue}
            onBack={goToPreviousStep}
          />
        );
      case "languageSelect":
        return (
          <LanguageSelectStep
            onContinue={handleLanguageSelectContinue}
            onBack={goToPreviousStep}
          />
        );
      case "learn":
        return (
          <LearnStep
            activationReached={learnActivated}
            onActivationReached={() => void recordOnboardingActivation("learn_mock_chat")}
            onComplete={handleLearnContinue}
            onBack={goToPreviousStep}
            onSkip={handleLearnSkip}
            userName={userName}
            modelReady={modelReady}
          />
        );
      case "success":
        return (
          <SuccessStep
            onComplete={handleSuccessComplete}
            onBack={goToPreviousStep}
          />
        );
      case "referral":
        return (
          <ReferralStep
            onComplete={handleReferralComplete}
            onBack={goToPreviousStep}
            userName={userName}
          />
        );
      default:
        return (
          <WelcomeStep
            onContinue={handleWelcomeContinue}
            initialName={userName}
          />
        );
    }
  };

  return (
    <>
      {renderStep()}
      {currentStep !== "downloadModel" && <ModelDownloadProgress />}
    </>
  );
};

export default Onboarding;
