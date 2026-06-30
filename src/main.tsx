import { ConfigProvider, theme } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { RouterProvider } from 'react-router';
import { router } from './app/router.tsx';
import './global.scss';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <ConfigProvider locale={ zhCN } theme={ {algorithm: theme.darkAlgorithm} }>
      <RouterProvider router={ router }/>
    </ConfigProvider>
  </React.StrictMode>,
);
