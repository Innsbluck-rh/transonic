import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, Show } from 'solid-js';
import { setSPNavStore, SPNavigationState, SPNavStore } from '~/stores/SPNavigationStore';

interface SPBottomNavigationProps {
  onClickNav?: (nav: SPNavigationState) => void;
}

const SPBottomNavigation: Component<SPBottomNavigationProps> = (props) => {
  return (
    <div class='border-primary-border bg-primary-plane z-20 flex flex-row border-t'>
      <SPBottomNavigationItem onClick={(nav) => props.onClickNav?.(nav)} navState={'index'} icon='material-symbols:library-music-sharp' />
      <SPBottomNavigationItem onClick={(nav) => props.onClickNav?.(nav)} navState={'setting'} icon='material-symbols:settings' />
      {/* <SPBottomNavigationItem onClick={(nav) => props.onClickNav?.(nav)} navState={'queue'} icon='material-symbols:playlist-play' label='queue' /> */}
    </div>
  );
};

export default SPBottomNavigation;

interface SPBottomNavigationItemProps {
  navState: SPNavigationState;
  icon: string;
  label?: string;
  onClick: (nav: SPNavigationState) => void;
}

const SPBottomNavigationItem: Component<SPBottomNavigationItemProps> = (props) => {
  const selected = createMemo<boolean>(() => SPNavStore.state === props.navState);
  return (
    <div
      class='ripple hover:bg-primary-hover flex flex-1 cursor-pointer flex-col items-center py-4'
      // classList={{ 'bg-primary-hover': selected() }}
      onClick={() => {
        props.onClick?.(props.navState);
        setSPNavStore('state', props.navState);
      }}
    >
      <Icon
        class='text-3xl'
        classList={{
          'text-accent': selected(),
          'text-secondary-text': !selected(),
        }}
        icon={props.icon}
      />
      <Show when={props.label?.trim()}>
        <p
          class='text-xs font-bold'
          classList={{
            'text-accent': selected(),
            'text-secondary-text': !selected(),
          }}
        >
          {props.label}
        </p>
      </Show>
    </div>
  );
};
