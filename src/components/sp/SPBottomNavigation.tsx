import { Icon } from '@iconify-icon/solid';
import { Component, createMemo } from 'solid-js';
import { setSPNavStore, SPNavigationState, SPNavStore } from '~/stores/SPNavigationStore';

interface SPBottomNavigationProps {}

const SPBottomNavigation: Component<SPBottomNavigationProps> = (_props) => {
  return (
    <div class='flex flex-row border-t border-primary-border'>
      <SPBottomNavigationItem navState={'index'} icon='material-symbols:library-music-sharp' label='index' />
      <SPBottomNavigationItem navState={'main'} icon='material-symbols:play-circle' label='main' />
      <SPBottomNavigationItem navState={'queue'} icon='material-symbols:playlist-play' label='queue' />
    </div>
  );
};

export default SPBottomNavigation;

interface SPBottomNavigationItemProps {
  navState: SPNavigationState;
  icon: string;
  label: string;
}

const SPBottomNavigationItem: Component<SPBottomNavigationItemProps> = (props) => {
  const selected = createMemo<boolean>(() => SPNavStore.state === props.navState);
  return (
    <div
      class='flex flex-col items-center flex-1 py-2.5 cursor-pointer hover:bg-primary-hover'
      classList={{ 'bg-primary-hover': selected() }}
      onClick={() => setSPNavStore('state', props.navState)}
    >
      <Icon
        class='text-3xl'
        classList={{
          'text-accent': selected(),
          'text-secondary-text': !selected(),
        }}
        icon={props.icon}
      />
      <p
        class='text-xs font-bold'
        classList={{
          'text-accent': selected(),
          'text-secondary-text': !selected(),
        }}
      >
        {props.label}
      </p>
    </div>
  );
};
