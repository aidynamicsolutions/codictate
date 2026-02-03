import React from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/shared/ui/card";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <Card className="w-full animate-in fade-in slide-in-from-bottom-2 duration-500 bg-card/60 backdrop-blur-sm border-border/60 hover:border-border/80 transition-colors">
      {(title || description) && (
        <CardHeader className="pb-3">
          {title && (
            <CardTitle className="text-sm font-semibold uppercase tracking-wide text-primary font-heading">
              {title}
            </CardTitle>
          )}
          {description && <CardDescription>{description}</CardDescription>}
        </CardHeader>
      )}
      <CardContent className="p-0">
        <div className="divide-y divide-border/60 dark:divide-white/15 px-6">
          {children}
        </div>
      </CardContent>
    </Card>
  );
};
