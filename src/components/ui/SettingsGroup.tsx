import React from "react";
import {
  Card,
  CardContent,
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
    <div className="w-full space-y-3">
      {(title || description) && (
        <div className="px-1">
          {title && (
            <h3 className="text-sm font-medium text-muted-foreground tracking-wide pl-1">
              {title}
            </h3>
          )}
          {description && (
            <p className="text-sm text-muted-foreground pl-1 mt-1">
              {description}
            </p>
          )}
        </div>
      )}
      <Card className="w-full bg-card border shadow-sm rounded-xl overflow-hidden">
        <CardContent className="p-0">
          <div className="divide-y divide-border/40 px-6">
            {children}
          </div>
        </CardContent>
      </Card>
    </div>
  );
};
