import { TextShimmer } from "@/components/TextShimmer";
import React from "react";

interface ChatStatusWidgetProps {
  text: string;
}

export const ChatStatusWidget: React.FC<ChatStatusWidgetProps> = ({ text }) => {
  return (
    <TextShimmer
      duration={1.2}
      className="font-medium [--base-color:#0bae59] [--base-gradient-color:#15d06e]"
    >
      {text}
    </TextShimmer>
  );
};
