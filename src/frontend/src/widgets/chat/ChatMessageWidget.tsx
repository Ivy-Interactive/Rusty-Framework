import { ChatBubble, ChatBubbleMessage } from "@/components/ChatBubble";
import React, { ReactNode } from "react";
import { User, LucideStars } from "lucide-react";
import { Densities } from "@/types/density";
import { cn } from "@/lib/utils";

export interface ChatMessageWidgetProps {
  id: number;
  children?: ReactNode[];
  sender: "User" | "Assistant";
  density?: Densities;
}

export const ChatMessageWidget: React.FC<ChatMessageWidgetProps> = ({
  id,
  sender = "User",
  children,
  density = Densities.Medium,
}) => {
  const avatarClass =
    density === Densities.Small
      ? "h-7 w-7 p-1.5"
      : density === Densities.Large
        ? "h-11 w-11 p-2.5"
        : "h-9 w-9 p-2";

  return (
    <ChatBubble key={id} variant={sender === "User" ? "sent" : "received"}>
      {sender == "Assistant" && (
        <div className={cn("bg-muted rounded-full flex items-center justify-center", avatarClass)}>
          <LucideStars />
        </div>
      )}

      {sender == "User" && (
        <div className={cn("bg-muted rounded-full flex items-center justify-center", avatarClass)}>
          <User />
        </div>
      )}

      <ChatBubbleMessage variant={sender === "User" ? "sent" : "received"}>
        <div>{children}</div>
      </ChatBubbleMessage>
    </ChatBubble>
  );
};

ChatMessageWidget.displayName = "ChatMessageWidget";
