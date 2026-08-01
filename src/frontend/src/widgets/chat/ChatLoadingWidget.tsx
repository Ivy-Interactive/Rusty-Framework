import { MessageLoading } from "@/components/MessageLoading";
import React from "react";

type ChatLoadingWidgetProps = Record<never, never>;

export const ChatLoadingWidget: React.FC<ChatLoadingWidgetProps> = () => {
  return <MessageLoading />;
};

ChatLoadingWidget.displayName = "ChatLoadingWidget";
