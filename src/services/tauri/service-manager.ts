const MOCK_OPERATION_DELAY = 550;

function simulateServiceOperation(serviceId: string): Promise<void> {
  return new Promise((resolve, reject) => {
    if (serviceId.trim().length === 0) {
      reject(new Error("A service identifier is required."));
      return;
    }

    setTimeout(resolve, MOCK_OPERATION_DELAY);
  });
}

export function startService(serviceId: string): Promise<void> {
  return simulateServiceOperation(serviceId);
}

export function stopService(serviceId: string): Promise<void> {
  return simulateServiceOperation(serviceId);
}

export function restartService(serviceId: string): Promise<void> {
  return simulateServiceOperation(serviceId);
}
