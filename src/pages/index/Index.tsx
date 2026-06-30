import { UploadOutlined, UserOutlined, VideoCameraOutlined } from '@ant-design/icons';
import { Layout, Menu, theme } from 'antd';
import { Outlet } from 'react-router';
import iconImg from '../../assets/icon.png';
import styles from './Index.module.scss';

const {Header, Sider, Content} = Layout;

const siderStyle: React.CSSProperties = {
  overflow: 'auto',
  height: '100vh',
  position: 'sticky',
  insetInlineStart: 0,
  top: 0,
  scrollbarWidth: 'thin',
  scrollbarGutter: 'stable',
};

export function IndexPage() {
  const {
    token: {colorBgContainer, borderRadiusLG},
  } = theme.useToken();

  return (
    <Layout>
      <Sider trigger={ null } width={ 200 } style={ {background: colorBgContainer} }>
        <div className={ styles.logoVertical }>
          <img src={ iconImg } alt="logo" style={ {width: 'auto', height: '64px'} }/>
        </div>
        <Menu
          mode="inline"
          defaultSelectedKeys={ ['1'] }
          items={ [
            {
              key: '1',
              icon: <UserOutlined/>,
              label: 'nav 1',
            },
            {
              key: '2',
              icon: <VideoCameraOutlined/>,
              label: 'nav 2',
            },
            {
              key: '3',
              icon: <UploadOutlined/>,
              label: 'nav 3',
            },
          ] }
        />
      </Sider>
      <Layout style={ {
        minHeight: '100vh',
      } }>
        <Header style={ {padding: 0, background: colorBgContainer} }/>
        <Content
          style={ {
            margin: '24px 16px',
            padding: 24,
            minHeight: 'auto',
            background: colorBgContainer,
            borderRadius: borderRadiusLG,
          } }>
          <Outlet/>
        </Content>
      </Layout>
    </Layout>
  );
}
