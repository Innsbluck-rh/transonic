import { Component, JSX } from 'solid-js';

const Title: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`archivo-black w-fit text-lg italic font-extrabold tracking-tighter scale-x-150 origin-top-left ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Title;
