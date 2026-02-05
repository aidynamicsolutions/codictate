import React, { Suspense } from "react";
import { useTranslation } from "react-i18next";
import { Cog, FlaskConical, History, Info, Sparkles, Home, Sliders } from "lucide-react";
import CodictateLogo from "./icons/CodictateLogo";
import { useSettings } from "../hooks/useSettings";
import HomeContent from "./home/Home";
import {
  Sidebar as SidebarPrimitive,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  SidebarSeparator,
  SidebarFooter,
} from "@/components/shared/ui/sidebar";
import ModelSelector from "./model-selector";
import {
  GeneralSettings,
  HistorySettings,
  DebugSettings,
  AboutSettings,
  PostProcessingSettings,
} from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  [key: string]: any;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType<any>;
  enabled: (settings: any) => boolean;
}

export const SECTIONS_CONFIG = {
  home: {
    labelKey: "sidebar.home",
    icon: Home,
    component: HomeContent,
    enabled: () => true,
  },
  settings: {
    labelKey: "sidebar.settings",
    icon: Cog,
    component: GeneralSettings,
    enabled: () => true,
  },
  postprocessing: {
    labelKey: "sidebar.postProcessing",
    icon: Sparkles,
    component: PostProcessingSettings,
    enabled: (settings) => settings?.post_process_enabled ?? false,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode ?? false,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const availableSections = Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  return (
    <SidebarPrimitive collapsible="icon" className="border-r border-mid-gray/20">
      <SidebarHeader className="flex flex-col items-center pt-4 pb-2 !h-[120px]">
        <CodictateLogo 
          className="fill-text stroke-text mb-2 group-data-[collapsible=icon]:mb-1 transition-all duration-300 w-20 group-data-[collapsible=icon]:w-10" 
        />
        <span className="font-bold text-xl tracking-tight group-data-[collapsible=icon]:text-[11px] group-data-[collapsible=icon]:mb-5">Codictate</span>
      </SidebarHeader>
      <SidebarSeparator className="mx-0" />
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {availableSections.map((section) => {
                const Icon = section.icon;
                const isActive = activeSection === section.id;

                return (
                  <SidebarMenuItem key={section.id}>
                    <SidebarMenuButton
                      isActive={isActive}
                      onClick={() => onSectionChange(section.id)}
                      tooltip={t(section.labelKey)}
                      size="lg"
                      className="text-base data-[active=true]:bg-logo-primary/80 data-[active=true]:text-white !h-11 group-data-[collapsible=icon]:!h-11 group-data-[collapsible=icon]:!w-full group-data-[collapsible=icon]:justify-center group-data-[collapsible=icon]:flex-col group-data-[collapsible=icon]:!px-0"
                    >
                      <Icon className="size-7 group-data-[collapsible=icon]:size-6" />
                      <span className="font-medium group-data-[collapsible=icon]:hidden">{t(section.labelKey)}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <div className="p-1">
          <ModelSelector />
        </div>
      </SidebarFooter>
      <SidebarRail />
    </SidebarPrimitive>
  );
};
