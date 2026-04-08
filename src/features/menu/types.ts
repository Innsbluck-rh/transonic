export interface MenuItem {
  icon?: string;
  label: string;
  onClick: () => void;
  disabled?: boolean;
}
