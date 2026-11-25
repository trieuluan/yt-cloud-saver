import React from 'react';
import { Composition } from 'remotion';
import { Tutorial } from './Tutorial';

export const RemotionRoot: React.FC = () => {
  return (
    <Composition
      id="Tutorial"
      component={Tutorial}
      durationInFrames={750}
      fps={30}
      width={1280}
      height={720}
    />
  );
};
