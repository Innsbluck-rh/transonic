import { Component } from 'solid-js';
import IndexContent from './IndexContent';

const IndexSideBar: Component = () => {
  return (
    <div class='flex flex-col min-w-40 w-56 border-r border-primary-border resize-x overflow-auto'>
      <IndexContent />
    </div>
  );
};

export default IndexSideBar;
