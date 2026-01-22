import React, { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Send, Slack } from "lucide-react";
import { Button } from "@/components/shared/ui/button";
import { commands } from "@/bindings";
import OnboardingLayout from "./OnboardingLayout";

interface LearnStepProps {
  onComplete: () => void;
  onBack?: () => void;
  onSkip?: () => void;
  userName?: string;
}

interface ChatMessage {
  id: string;
  sender: "bot" | "user";
  text: string;
}

// Helper to get canned response from i18n array
const getCannedResponse = (
  t: (key: string, options?: { returnObjects: boolean }) => string | string[],
  index: number
): string => {
  const responses = t("onboarding.learn.cannedResponses", {
    returnObjects: true,
  }) as string[];
  // First 4 are sequential, rest cycle
  if (index < 4) return responses[index];
  return responses[4 + ((index - 4) % (responses.length - 4))];
};

// Typing indicator component
const TypingIndicator: React.FC = () => (
  <div className="flex items-center gap-1 px-3 py-2">
    <div className="flex gap-1">
      <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce [animation-delay:0ms]" />
      <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce [animation-delay:150ms]" />
      <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce [animation-delay:300ms]" />
    </div>
  </div>
);

// Avatar component for bot
const BotAvatar: React.FC = () => (
  <img
    src="/src-tauri/resources/botAvatar.png"
    alt="Alex"
    className="w-10 h-10 rounded-full object-cover"
  />
);

// Avatar component for user (initials)
const UserAvatar: React.FC<{ name: string }> = ({ name }) => {
  const initials = name
    .split(" ")
    .map((n) => n[0])
    .join("")
    .toUpperCase()
    .slice(0, 2) || "U";

  return (
    <div className="w-10 h-10 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-sm font-medium">
      {initials}
    </div>
  );
};

// Chat message component
const ChatMessageBubble: React.FC<{
  message: ChatMessage;
  userName: string;
}> = ({ message, userName }) => {
  const { t } = useTranslation();
  const isBot = message.sender === "bot";

  return (
    <div className="flex items-start gap-3 mb-4">
      {isBot ? <BotAvatar /> : <UserAvatar name={userName} />}
      <div className="flex flex-col">
        <span className="text-sm font-semibold text-gray-900 mb-1">
          {isBot ? t("onboarding.learn.botName") : t("onboarding.learn.you")}
        </span>
        <p className="text-sm text-gray-700 leading-relaxed">{message.text}</p>
      </div>
    </div>
  );
};


export const LearnStep: React.FC<LearnStepProps> = ({
  onComplete,
  onBack,
  onSkip,
  userName = "",
}) => {
  const { t } = useTranslation();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const [responseIndex, setResponseIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const chatContainerRef = useRef<HTMLDivElement>(null);

  // Display name - use provided name or fallback
  const displayName = userName || t("onboarding.attribution.defaultName");

  // Initialize with Alex's greeting
  useEffect(() => {
    const greeting = t("onboarding.learn.botMessages.greeting", {
      name: displayName,
    });
    setMessages([
      {
        id: "greeting",
        sender: "bot",
        text: greeting,
      },
    ]);
  }, [displayName, t]);

  // Enable Direct paste override for onboarding
  // This works around WebView not receiving CGEvent-simulated Cmd+V keystrokes
  useEffect(() => {
    commands.setOnboardingPasteOverride(true);
    return () => {
      commands.setOnboardingPasteOverride(false);
    };
  }, []);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (chatContainerRef.current) {
      chatContainerRef.current.scrollTop = chatContainerRef.current.scrollHeight;
    }
  }, [messages, isTyping]);

  // Focus input on mount
  useEffect(() => {
    // Small delay to ensure the input is rendered
    const timer = setTimeout(() => {
      inputRef.current?.focus();
    }, 100);
    return () => clearTimeout(timer);
  }, []);

  // Handle user message submission
  const handleSubmit = useCallback(() => {
    if (!inputValue.trim()) return;

    // Add user message
    const userMessage: ChatMessage = {
      id: `user-${Date.now()}`,
      sender: "user",
      text: inputValue.trim(),
    };
    setMessages((prev) => [...prev, userMessage]);
    setInputValue("");

    // Show typing indicator after a short delay
    setTimeout(() => {
      setIsTyping(true);
    }, 300);

    // Add bot response after typing animation
    setTimeout(() => {
      setIsTyping(false);
      const response = getCannedResponse(t, responseIndex);
      const botMessage: ChatMessage = {
        id: `bot-${Date.now()}`,
        sender: "bot",
        text: response,
      };
      setMessages((prev) => [...prev, botMessage]);
      setResponseIndex((prev) => prev + 1);

      // Refocus input after bot response
      setTimeout(() => {
        inputRef.current?.focus();
      }, 100);
    }, 1500 + Math.random() * 1000); // 1.5-2.5 second typing delay
  }, [inputValue, responseIndex, t]);

  // Handle input changes
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setInputValue(e.target.value);
  };

  // Handle Enter key to submit
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && inputValue.trim()) {
      e.preventDefault();
      handleSubmit();
    }
  };

  // Handle skip - go directly to completion
  const handleSkip = () => {
    if (onSkip) {
      onSkip();
    } else {
      onComplete();
    }
  };

  return (
    <OnboardingLayout
      currentStep="learn"
      leftContent={
        <div className="flex flex-col h-full">
          {/* Back button - positioned at top */}
          {onBack && (
            <button
              type="button"
              onClick={onBack}
              className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors w-fit mb-auto"
            >
              <ArrowLeft className="h-4 w-4" />
              {t("onboarding.learn.back")}
            </button>
          )}

          {/* Content centered vertically */}
          <div className="flex flex-col gap-4 my-auto max-w-[380px]">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
              {t("onboarding.learn.title")}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.learn.subtitle")}{" "}
              <span className="inline-block px-1.5 py-0.5 bg-muted rounded text-foreground font-medium text-sm border border-border">
                {t("onboarding.hotkeySetup.subtitleFnKey")}
              </span>{" "}
              {t("onboarding.learn.subtitleEnd")}
            </p>
          </div>

          {/* Buttons at bottom */}
          <div className="flex items-center gap-3 mt-auto">
            <Button onClick={onComplete} size="lg">
              {t("onboarding.learn.complete")}
            </Button>
            <button
              type="button"
              onClick={handleSkip}
              className="text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              {t("onboarding.learn.skip")}
            </button>
          </div>
        </div>
      }
      rightContent={
        <div className="flex items-center justify-center h-full w-full px-8">
          {/* Slack Chat Card */}
          <div className="bg-white rounded-xl shadow-lg overflow-hidden w-full min-w-[520px]">
            {/* Slack Header */}
            <div className="bg-[#4A154B] text-white px-4 py-3 flex items-center gap-2">
              <Slack className="w-5 h-5" />
              <span className="font-semibold text-sm">
                {t("onboarding.learn.slackTitle")}
              </span>
            </div>

            {/* Chat Messages */}
            <div
              ref={chatContainerRef}
              className="p-4 h-[280px] overflow-y-auto bg-gray-50"
            >
              {messages.map((message) => (
                <ChatMessageBubble
                  key={message.id}
                  message={message}
                  userName={displayName}
                />
              ))}
              {isTyping && (
                <div className="flex items-start gap-3 mb-4">
                  <BotAvatar />
                  <div className="flex flex-col">
                    <span className="text-sm font-semibold text-gray-900 mb-1">
                      {t("onboarding.learn.botName")}
                    </span>
                    <TypingIndicator />
                  </div>
                </div>
              )}
            </div>

            {/* Input Area */}
            <div className="border-t border-gray-200 p-3">
              <div className="flex items-center gap-2 bg-gray-100 rounded-lg px-3 py-2">
                <input
                  ref={inputRef}
                  type="text"
                  value={inputValue}
                  onChange={handleInputChange}
                  onKeyDown={handleKeyDown}
                  placeholder={t("onboarding.learn.inputPlaceholder")}
                  className="flex-1 bg-transparent text-sm text-gray-700 placeholder-gray-400 outline-none"
                />
                <button
                  type="button"
                  onClick={handleSubmit}
                  disabled={!inputValue.trim()}
                  className="text-gray-400 hover:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                  <Send className="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>
        </div>
      }
    />
  );
};

export default LearnStep;
