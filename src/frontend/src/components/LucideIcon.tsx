import { icons } from "lucide-react";

interface LucideIconProps {
  name: string;
  color?: string;
  size?: string | number;
  className?: string;
  style?: React.CSSProperties;
}

const LucideIcon: React.FC<LucideIconProps> = ({ name, color, size, className, style }) => {
  if (!(name in icons)) {
    return null;
  }
  const Cmp = icons[name as keyof typeof icons];
  return <Cmp style={style} color={color} size={size} className={className} />;
};

export default LucideIcon;
