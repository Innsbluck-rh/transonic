import { Component } from 'solid-js';

type PlayerIconTypes = 'play' | 'pause' | 'next' | 'prev';

interface PlayerIconProps {
  type: PlayerIconTypes;
}

const PlayerIcon: Component<PlayerIconProps> = (props) => {
  return (
    <div class='flex items-center justify-center w-12 h-12'>
      <img
        class='scale-200 cursor-pointer'
        src={getIconSrc(props.type)}
        style={{
          'image-rendering': 'pixelated',
        }}
      />
    </div>
  );
};

function getIconSrc(type: PlayerIconTypes) {
  switch (type) {
    case 'play':
      return '/play.webp';
    case 'pause':
      return '/pause.webp';
    case 'next':
      return '/play_next.webp';
    case 'prev':
      return '/play_prev.webp';
  }
}

export default PlayerIcon;
