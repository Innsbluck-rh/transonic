import { Component } from 'solid-js';
import SeekSlider from '../common/SeekSlider';

interface PlayerSliderProps {
  valueMs?: number;
  maxMs?: number;
  disabled?: boolean;
  onPreview?: (valueMs: number | null) => void;
  onCommit?: (valueMs: number) => void;
}

const PlayerSlider: Component<PlayerSliderProps> = (props) => {
  return (
    <SeekSlider
      value={props.valueMs}
      max={props.maxMs}
      disabled={props.disabled}
      onPreview={props.onPreview}
      onCommit={props.onCommit}
      rootClass='group relative w-full h-2 touch-none'
      hitAreaClass='absolute top-1/2 left-0 w-full h-2 -translate-y-1/2'
      trackClass='absolute top-1/2 left-0 w-full h-0.5 -translate-y-1/2 bg-secondary-border'
      progressClass='absolute top-1/2 left-0 h-0.5 -translate-y-1/2 bg-accent'
      handleClass='absolute top-1/2 origin-center w-2 h-2 rounded-full bg-accent transition-opacity translate-x-[-50%] -translate-y-1/2 opacity-0 group-hover:opacity-100'
    />
  );
};

export default PlayerSlider;
