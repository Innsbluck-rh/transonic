import { Component } from 'solid-js';
import Heading3 from '~/components/common/Heading3';
import QueueContent from './QueueContent';

const QueueSideBar: Component = () => {
  return (
    <div class='flex flex-col min-w-40 w-56 bg-primary-plane border-l border-primary-border resize-x overflow-auto'>
      <div class='flex h-6 w-full flex-row items-center px-1 border-b border-secondary-border'>
        <Heading3 class='w-fit text-secondary-text'>Playback Queue</Heading3>
      </div>
      <QueueContent />
    </div>
  );
};

export default QueueSideBar;
